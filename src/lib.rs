use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
use github_flows::{get_octo, octocrab::models::issues::Issue, GithubLogin};
use openai_flows::{
    chat::{ChatModel, ChatOptions},
    OpenAIFlows,
};
use regex::Regex;
use serde::Deserialize;
use serde_json;
use slack_flows::{listen_to_channel, send_message_to_channel, SlackMessage};
use std::env;

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn run() {
    dotenv().ok();
    logger::init();
    let slack_workspace = env::var("slack_workspace").unwrap_or("secondstate".to_string());
    let slack_channel = env::var("slack_channel").unwrap_or("test-flow".to_string());

    listen_to_channel(&slack_workspace, &slack_channel, |sm| {
        handler(&slack_workspace, &slack_channel, sm)
    })
    .await;
}

#[no_mangle]
async fn handler(worksapce: &str, channel: &str, sm: SlackMessage) {
    let trigger_word = env::var("trigger_word").unwrap_or("flows summarize".to_string());
    let octocrab = get_octo(&GithubLogin::Default);
    let re = Regex::new(r"^(\s*\w+(?: \w+)?)(.*)( \d+)").unwrap();
    let cap = re.captures(&sm.text).unwrap();

    let triggered = match cap.get(1) {
        Some(trigger) => trigger.as_str().trim().contains(&trigger_word),
        None => false,
    };

    if !triggered {
        return;
    }

    let _n_days = match cap.get(3) {
        Some(n) => n.as_str().trim().parse::<i64>().unwrap_or(7),
        None => 7,
    };

    if let Some(owner_repo_str) = cap.get(2) {
        let owner_repo = owner_repo_str
            .as_str()
            .trim()
            .split("/")
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        let owner = owner_repo
            .get(0)
            .unwrap_or(&"WasmEdge".to_string())
            .to_string();
        let repo = owner_repo
            .get(1)
            .unwrap_or(&"Wasmdge".to_string())
            .to_string();

        let n_days_ago_str =
            (chrono::Utc::now() - chrono::Duration::days(_n_days)).format("%Y-%m-%dT%H:%M:%SZ");
        let query = format!("repo:{owner}/{repo} is:issue state:open updated:>{n_days_ago_str}");
        match octocrab
            .search()
            .issues_and_pull_requests(&query)
            .sort("desc")
            .order("updated")
            .per_page(100)
            .page(1u32)
            .send()
            .await
        {
            Ok(issues_on_target) => {
                let mut count = 10;

                for issue in issues_on_target.items {
                    count -= 1;

                    let summary = match analyze_issue(&owner, &repo, issue.clone()).await {
                        Some(s) => format!("{}\n{}", s, issue.html_url),
                        None => format!(
                            "Summarization failed, no summary generated for issue: {}",
                            issue.html_url
                        ),
                    };

                    send_message_to_channel(&worksapce, &channel, summary.to_string()).await;
                    if count <= 0 {
                        send_message_to_channel(
                                &worksapce,
                                &channel,
                                "You've reached your limit of 10 issues. Please wait 10 minutes before running the command again.".to_string(),
                            ).await;
                        break;
                    }
                }
            }

            Err(_e) => {
                log::error!("Error getting issues from target: {}", _e);
                if triggered {
                    let _text = sm.text.clone();
                    send_message_to_channel(
                        &worksapce,
                        &channel,
                        format!(
                            r#"Please double check if there are errors in the owner and repo names provided in your message:
{_text}
if yes, please correct the spelling and resend your instruction."#
                        ),
                    ).await;
                    return;
                }
            }
        }
    }
}

pub fn squeeze_fit_remove_quoted(inp_str: &str, max_len: u16, split: f32) -> String {
    let mut body = String::new();
    let mut inside_quote = false;

    for line in inp_str.lines() {
        if line.contains("```") || line.contains("\"\"\"") {
            inside_quote = !inside_quote;
            continue;
        }

        if !inside_quote {
            let cleaned_line = line
                .split_whitespace()
                .filter(|word| word.len() < 150)
                .collect::<Vec<&str>>()
                .join(" ");
            body.push_str(&cleaned_line);
            body.push('\n');
        }
    }

    let body_words: Vec<&str> = body.split_whitespace().collect();
    let body_len = body_words.len();
    let n_take_from_beginning = (body_len as f32 * split) as usize;
    let n_keep_till_end = body_len - n_take_from_beginning;

    // Range check for drain operation
    let drain_start = if n_take_from_beginning < body_len {
        n_take_from_beginning
    } else {
        body_len
    };

    let drain_end = if n_keep_till_end <= body_len {
        body_len - n_keep_till_end
    } else {
        0
    };

    let final_text = if body_len > max_len as usize {
        let mut body_text_vec = body_words.to_vec();
        body_text_vec.drain(drain_start..drain_end);
        body_text_vec.join(" ")
    } else {
        body
    };

    final_text
}

pub fn squeeze_fit_post_texts(inp_str: &str, max_len: u16, split: f32) -> String {
    let bpe = tiktoken_rs::cl100k_base().unwrap();

    let input_token_vec = bpe.encode_ordinary(inp_str);
    let input_len = input_token_vec.len();
    if input_len < max_len as usize {
        return inp_str.to_string();
    }
    let n_take_from_beginning = (input_len as f32 * split).ceil() as usize;
    let n_take_from_end = max_len as usize - n_take_from_beginning;

    let mut concatenated_tokens = Vec::with_capacity(max_len as usize);
    concatenated_tokens.extend_from_slice(&input_token_vec[..n_take_from_beginning]);
    concatenated_tokens.extend_from_slice(&input_token_vec[input_len - n_take_from_end..]);

    bpe.decode(concatenated_tokens)
        .ok()
        .map_or("failed to decode tokens".to_string(), |s| s.to_string())
}

pub async fn analyze_issue(owner: &str, repo: &str, issue: Issue) -> Option<String> {
    let mut openai = OpenAIFlows::new();
    openai.set_retry_times(2);
    let octocrab = get_octo(&GithubLogin::Default);

    let issue_creator_name = &issue.user.login;
    let issue_title = issue.title.to_string();
    let issue_number = issue.number;

    let issue_body = match &issue.body {
        Some(body) => squeeze_fit_remove_quoted(body, 500, 0.6),
        None => "".to_string(),
    };

    let labels = issue
        .labels
        .iter()
        .map(|lab| lab.name.clone())
        .collect::<Vec<String>>()
        .join(", ");

    let mut all_text_from_issue = format!(
        "User '{}', opened an issue titled '{}', labeled '{}', with the following post: '{}'.",
        issue_creator_name, issue_title, labels, issue_body
    );

    match octocrab
        .issues(owner, repo)
        .list_comments(issue_number)
        .per_page(100)
        .page(1u32)
        .send()
        .await
    {
        Ok(comments_page) => {
            for comment in comments_page.items {
                let comment_body = match &comment.body {
                    Some(body) => squeeze_fit_remove_quoted(body, 300, 0.6),
                    None => "".to_string(),
                };
                let commenter = &comment.user.login;
                let commenter_input = format!("{} commented: {}", commenter, comment_body);

                all_text_from_issue.push_str(&commenter_input);
            }
        }

        Err(_e) => log::error!("Error getting comments from issue: {}", _e),
    };

    let all_text_from_issue = squeeze_fit_post_texts(&all_text_from_issue, 12_000, 0.4);

    // let sys_prompt_1 = &format!(
    //     "Given the information that user '{issue_creator_name}' opened an issue titled '{issue_title}', your task is to deeply analyze the content of the issue posts. Distill the crux of the issue, the potential solutions suggested.Concentrate on the principal arguments, suggested solutions, and areas of consensus or disagreement among the participants. From these elements, generate a concise summary of the entire issue to inform the next course of action."
    // );
    let sys_prompt_1 = &format!(
        "Given the information that user '{issue_creator_name}' opened an issue titled '{issue_title}', your task is to deeply analyze the content of the issue posts. Concentrate on the principal arguments, suggested solutions, and areas of consensus or disagreement among the participants, then generate a succinct, context-aware summary of the issue."
    );

    let co = match all_text_from_issue.len() > 12000 {
        true => ChatOptions {
            model: ChatModel::GPT35Turbo16K,
            system_prompt: Some(sys_prompt_1),
            restart: true,
            temperature: Some(0.7),
            max_tokens: Some(256),
            ..Default::default()
        },
        false => ChatOptions {
            model: ChatModel::GPT35Turbo,
            system_prompt: Some(sys_prompt_1),
            restart: true,
            temperature: Some(0.7),
            max_tokens: Some(256),
            ..Default::default()
        },
    };
    // let usr_prompt_1 = &format!(
    //     "Analyze the GitHub issue content: {all_text_from_issue}. Provide a concise analysis touching upon: The central problem discussed in the issue. The main solutions proposed or agreed upon. Aim for a succinct, analytical summary that stays under 128 tokens."
    // );

    let usr_prompt_1 = &format!(
        "Analyze the GitHub issue content: {}. \
        Concentrate on the principal arguments, suggested solutions, and areas of consensus or \
        disagreement among the participants. \
        From these elements, generate a concise summary of the entire issue to inform the next course of action. \
        Please reply in the following JSON format, leaving fields empty if no relevant information is generated: \n\n\
        ```\n\
        {{\n\
          \"PrincipalArguments\": \"List principal arguments here as complete sentences.\",\n\
          \"SuggestedSolutions\": \"List any suggested solutions here as complete sentences.\",\n\
          \"AreasOfConsensus\": \"Highlight areas where there's general agreement among the participants as complete sentences.\",\n\
          \"AreasOfDisagreement\": \"Highlight areas where there's general disagreement among the participants as complete sentences.\",\n\
          \"ConciseSummary\": \"Provide a concise, analytical summary of the issue as a complete sentence.\"\n\
        }}\n\
        ```",
        all_text_from_issue
    );

    match openai
        .chat_completion(&format!("issue_{issue_number}"), usr_prompt_1, &co)
        .await
    {
        Ok(r) => match extract_and_parse_summary(&r.choice) {
            Some(parsed_summary) => {
                let arguments = match parsed_summary.PrincipalArguments {
                    Some(v) => format!("Key arguments: {:?}\n", v.join(" ")),
                    None => String::new(),
                };
                let solutions = match parsed_summary.SuggestedSolutions {
                    Some(v) => format!("Solutions: {:?}\n", v.join(" ")),
                    None => String::new(),
                };
                let consensus = match parsed_summary.AreasOfConsensus {
                    Some(v) => format!("Consensus: {:?}\n", v.join(" ")),
                    None => String::new(),
                };
                let disagreement = match parsed_summary.AreasOfDisagreement {
                    Some(v) => format!("Disagreement: {:?}\n", v.join(" ")),
                    None => String::new(),
                };
                let summary = match parsed_summary.ConciseSummary {
                    Some(s) => format!("Summary: {}", s),
                    None => String::new(),
                };
                let out = format!(
                    "{} {} {} {} {}",
                    arguments, solutions, consensus, disagreement, summary
                );

                Some(out)
            }
            None => {
                log::error!("Error generating issue summary #{}", issue_number);
                None
            }
        },

        Err(_e) => {
            log::error!("Error generating issue summary #{}: {}", issue_number, _e);
            None
        }
    }
}

use std::ops::Range;

#[derive(Debug, Deserialize)]
struct GitHubIssueSummary {
    PrincipalArguments: Option<Vec<String>>,
    SuggestedSolutions: Option<Vec<String>>,
    AreasOfConsensus: Option<Vec<String>>,
    AreasOfDisagreement: Option<Vec<String>>,
    ConciseSummary: Option<String>,
}

fn find_json_range(text: &str) -> Option<Range<usize>> {
    let mut depth = 0;
    let mut start = None;
    let mut end = None;
    for (i, c) in text.chars().enumerate() {
        if c == '{' {
            if depth == 0 {
                start = Some(i);
            }
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                end = Some(i + 1);
                break;
            }
        }
    }
    if let (Some(start), Some(end)) = (start, end) {
        Some(start..end)
    } else {
        None
    }
}

fn extract_and_parse_summary(input: &str) -> Option<GitHubIssueSummary> {
    let json_range = find_json_range(input)?;
    let json_str = &input[json_range];
    
    match serde_json::from_str::<GitHubIssueSummary>(json_str) {
        Ok(s) => Some(s),
        Err(err) => {
            log::error!("Error parsing issue summary: {}", err);
            None
        }
    }
}