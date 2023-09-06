# <p align="center">Summarize GitHub Issues from the Past 7 Days</p>

<p align="center">
  <a href="https://discord.gg/ccZn9ZMfFf">
    <img src="https://img.shields.io/badge/chat-Discord-7289DA?logo=discord" alt="flows.network Discord">
  </a>
  <a href="https://twitter.com/flows_network">
    <img src="https://img.shields.io/badge/Twitter-1DA1F2?logo=twitter&amp;logoColor=white" alt="flows.network Twitter">
  </a>
   <a href="https://flows.network/flow/createByTemplate/github-issues-report-to-slack">
    <img src="https://img.shields.io/website?up_message=deploy&url=https%3A%2F%2Fflows.network%2Fflow%2Fnew" alt="Create a flow">
  </a>
</p>

Stay informed on the issues your open-source project users encounter. Our bot summarizes issues from your GitHub repository over the past 7 days (or fewer) and shares each issue summary as a separate Slack message, streamlining your review process.

This bot can work with any public GitHub repos.

## How to Use:

To trigger this bot, send a message to the designated Slack channel with `trigger_word` <github_owner>/<github_repo> [n].
- `trigger_word` are the word(s) of your choice. It defaults to "flows summarize" if not specified when you create the flow.
- github_owner is the owner of the GitHub repository
- github_repo is the GitHub repository to summarize
  - github_owner and github_repo are separated by a '/'
- `[n]`: Number of days for the summary (default is 7 days if left empty).

Let's take Pytorch as an example. Send `flows summarize pytorch/pytorch` to the designated channel, you will receive summaries of each issue raised in the last 7 days.

## Deploy on your own repo

1. Create a bot from the template
2. Add your OpenAI API key
3. Authenticate Slack
4. Authenticate GitHub

### 0 Prerequisites

You will need to bring your own [OpenAI API key](https://openai.com/blog/openai-api). If you do not already have one, [sign up here](https://platform.openai.com/signup).

You will also need to sign into [flows.network](https://flows.network/) using your GitHub account. It is free to join.

### 1 Create a bot from the template

[**Just click here**](https://flows.network/flow/createByTemplate/github-issues-report-to-slack)

Review the `trigger_phrase`. 

* Set your `trigger_phrase`. By default, it is `flows summarize`.

Click **Create and Build**.

### 2 Add your OpenAI API key
You will now set up OpenAI integration. Click **Connect**, enter your key and name it.

[<img width="450" alt="image" src="https://user-images.githubusercontent.com/45785633/222973214-ecd052dc-72c2-4711-90ec-db1ec9d5f24e.png">](https://user-images.githubusercontent.com/45785633/222973214-ecd052dc-72c2-4711-90ec-db1ec9d5f24e.png)

Once added, close the tab and return to flows.network. Click on **Continue**.

### 3 Authenticate Slack

Next, you will tell the bot which Slack channel you want your summary to be sent to.

* `slack_channel`: Slack organization of the Slack channel where you want to deploy the bot. Case sensitive.

* `slack_workspace`: The Slack channel where you want to deploy the bot. Case sensitive.

Enter your Slack workspace and channel respectively in the red boxes below.
![image](https://github.com/flows-network/github-star-slack-messenger/assets/45785633/0d9ac244-f327-4366-972c-47ef05472057)

| Name           | Value               |
|----------------|---------------------|
| `slack_workspace` | flowsnetwork    |
| `slack_channel`  | gihtub-issues |

Click the "Connect/+ Add new authentication" button to authenticate your Slack account. You'll be redirected to a new page where you must grant [flows.network](https://flows.network/) permission to install the `flows-network` bot on your workspace. This workspace is the one you entered into the `slack_workspace` above. The Slack channel must be public.

Once added, close the tab and return to flows.network. Click **Continue**.

### 3 Authenticate GitHub

Click **Connect** or **+ Add new authentication** to grant [flows.network](https://flows.network/) your GitHub access. The bot can connect any public GitHub repo.

[<img width="450" alt="image" src="https://github.com/flows-network/github-pr-summary/assets/45785633/6cefff19-9eeb-4533-a20b-03c6a9c89473">](https://github.com/flows-network/github-pr-summary/assets/45785633/6cefff19-9eeb-4533-a20b-03c6a9c89473)

Once done, close the popup window and return to the flow.network page. Click **Deploy**.

### Wait for the magic!

This is it! You are now on the flow details page waiting for the flow function to build. As soon as the flow's status becomes `running`, the bot is ready to summarize issues! Comment the trigger phrase in Slack to trigger it.

## Note
- Summarization might take longer for repositories with extensive discussions or large comments.
- Each request will provide individual summaries for a maximum of 10 issues.
