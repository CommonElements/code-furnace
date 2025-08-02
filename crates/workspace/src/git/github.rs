use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRepository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub private: bool,
    pub html_url: String,
    pub clone_url: String,
    pub ssh_url: String,
    pub default_branch: String,
    pub language: Option<String>,
    pub stargazers_count: u32,
    pub forks_count: u32,
    pub open_issues_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPullRequest {
    pub id: u64,
    pub number: u32,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub draft: bool,
    pub html_url: String,
    pub user: GitHubUser,
    pub head: GitHubBranch,
    pub base: GitHubBranch,
    pub created_at: String,
    pub updated_at: String,
    pub mergeable: Option<bool>,
    pub merged: bool,
    pub comments: u32,
    pub review_comments: u32,
    pub commits: u32,
    pub additions: u32,
    pub deletions: u32,
    pub changed_files: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIssue {
    pub id: u64,
    pub number: u32,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub html_url: String,
    pub user: GitHubUser,
    pub assignees: Vec<GitHubUser>,
    pub labels: Vec<GitHubLabel>,
    pub milestone: Option<GitHubMilestone>,
    pub created_at: String,
    pub updated_at: String,
    pub comments: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub id: u64,
    pub login: String,
    pub avatar_url: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubBranch {
    pub label: String,
    pub r#ref: String,
    pub sha: String,
    pub repo: Option<GitHubRepository>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubLabel {
    pub id: u64,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubMilestone {
    pub id: u64,
    pub number: u32,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubWorkflowRun {
    pub id: u64,
    pub name: String,
    pub head_branch: String,
    pub head_sha: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub workflow_id: u64,
    pub html_url: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub id: u64,
    pub tag_name: String,
    pub name: String,
    pub body: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    pub html_url: String,
    pub published_at: Option<String>,
    pub author: GitHubUser,
}

pub struct GitHubClient {
    client: Client,
    token: Option<String>,
    base_url: String,
}

impl GitHubClient {
    pub fn new(token: Option<String>) -> Self {
        Self {
            client: Client::new(),
            token,
            base_url: "https://api.github.com".to_string(),
        }
    }
    
    pub fn with_enterprise_url(token: Option<String>, base_url: String) -> Self {
        Self {
            client: Client::new(),
            token,
            base_url,
        }
    }
    
    async fn make_request<T>(&self, endpoint: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{}/{}", self.base_url, endpoint);
        let mut request = self.client.get(&url);
        
        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("token {}", token));
        }
        
        request = request.header("User-Agent", "Code-Furnace/1.0");
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }
        
        let json: T = response.json().await?;
        Ok(json)
    }
    
    async fn make_post_request<T, B>(&self, endpoint: &str, body: &B) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
        B: Serialize,
    {
        let url = format!("{}/{}", self.base_url, endpoint);
        let mut request = self.client.post(&url);
        
        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("token {}", token));
        }
        
        request = request
            .header("User-Agent", "Code-Furnace/1.0")
            .header("Content-Type", "application/json")
            .json(body);
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }
        
        let json: T = response.json().await?;
        Ok(json)
    }
    
    pub async fn get_repository(&self, owner: &str, repo: &str) -> Result<GitHubRepository> {
        let endpoint = format!("repos/{}/{}", owner, repo);
        self.make_request(&endpoint).await
    }
    
    pub async fn list_pull_requests(
        &self, 
        owner: &str, 
        repo: &str, 
        state: Option<&str>, 
        base: Option<&str>
    ) -> Result<Vec<GitHubPullRequest>> {
        let mut endpoint = format!("repos/{}/{}/pulls", owner, repo);
        let mut params = Vec::new();
        
        if let Some(state) = state {
            params.push(format!("state={}", state));
        }
        
        if let Some(base) = base {
            params.push(format!("base={}", base));
        }
        
        if !params.is_empty() {
            endpoint.push('?');
            endpoint.push_str(&params.join("&"));
        }
        
        self.make_request(&endpoint).await
    }
    
    pub async fn get_pull_request(&self, owner: &str, repo: &str, number: u32) -> Result<GitHubPullRequest> {
        let endpoint = format!("repos/{}/{}/pulls/{}", owner, repo, number);
        self.make_request(&endpoint).await
    }
    
    pub async fn create_pull_request(
        &self, 
        owner: &str, 
        repo: &str, 
        title: &str, 
        body: Option<&str>, 
        head: &str, 
        base: &str,
        draft: bool
    ) -> Result<GitHubPullRequest> {
        let endpoint = format!("repos/{}/{}/pulls", owner, repo);
        
        let mut pr_data = HashMap::new();
        pr_data.insert("title", title);
        pr_data.insert("head", head);
        pr_data.insert("base", base);
        
        if let Some(body) = body {
            pr_data.insert("body", body);
        }
        
        if draft {
            pr_data.insert("draft", "true");
        }
        
        self.make_post_request(&endpoint, &pr_data).await
    }
    
    pub async fn list_issues(
        &self, 
        owner: &str, 
        repo: &str, 
        state: Option<&str>,
        labels: Option<&str>
    ) -> Result<Vec<GitHubIssue>> {
        let mut endpoint = format!("repos/{}/{}/issues", owner, repo);
        let mut params = Vec::new();
        
        if let Some(state) = state {
            params.push(format!("state={}", state));
        }
        
        if let Some(labels) = labels {
            params.push(format!("labels={}", labels));
        }
        
        if !params.is_empty() {
            endpoint.push('?');
            endpoint.push_str(&params.join("&"));
        }
        
        self.make_request(&endpoint).await
    }
    
    pub async fn get_issue(&self, owner: &str, repo: &str, number: u32) -> Result<GitHubIssue> {
        let endpoint = format!("repos/{}/{}/issues/{}", owner, repo, number);
        self.make_request(&endpoint).await
    }
    
    pub async fn create_issue(
        &self, 
        owner: &str, 
        repo: &str, 
        title: &str, 
        body: Option<&str>,
        labels: Option<Vec<&str>>,
        assignees: Option<Vec<&str>>
    ) -> Result<GitHubIssue> {
        let endpoint = format!("repos/{}/{}/issues", owner, repo);
        
        let mut issue_data = HashMap::new();
        issue_data.insert("title", serde_json::Value::String(title.to_string()));
        
        if let Some(body) = body {
            issue_data.insert("body", serde_json::Value::String(body.to_string()));
        }
        
        if let Some(labels) = labels {
            issue_data.insert("labels", serde_json::Value::Array(
                labels.into_iter().map(|l| serde_json::Value::String(l.to_string())).collect()
            ));
        }
        
        if let Some(assignees) = assignees {
            issue_data.insert("assignees", serde_json::Value::Array(
                assignees.into_iter().map(|a| serde_json::Value::String(a.to_string())).collect()
            ));
        }
        
        self.make_post_request(&endpoint, &issue_data).await
    }
    
    pub async fn list_workflow_runs(&self, owner: &str, repo: &str, branch: Option<&str>) -> Result<Vec<GitHubWorkflowRun>> {
        let mut endpoint = format!("repos/{}/{}/actions/runs", owner, repo);
        
        if let Some(branch) = branch {
            endpoint.push_str(&format!("?branch={}", branch));
        }
        
        #[derive(Deserialize)]
        struct WorkflowRunsResponse {
            workflow_runs: Vec<GitHubWorkflowRun>,
        }
        
        let response: WorkflowRunsResponse = self.make_request(&endpoint).await?;
        Ok(response.workflow_runs)
    }
    
    pub async fn list_releases(&self, owner: &str, repo: &str) -> Result<Vec<GitHubRelease>> {
        let endpoint = format!("repos/{}/{}/releases", owner, repo);
        self.make_request(&endpoint).await
    }
    
    pub async fn get_latest_release(&self, owner: &str, repo: &str) -> Result<GitHubRelease> {
        let endpoint = format!("repos/{}/{}/releases/latest", owner, repo);
        self.make_request(&endpoint).await
    }
    
    pub async fn create_release(
        &self,
        owner: &str,
        repo: &str,
        tag_name: &str,
        name: &str,
        body: Option<&str>,
        draft: bool,
        prerelease: bool
    ) -> Result<GitHubRelease> {
        let endpoint = format!("repos/{}/{}/releases", owner, repo);
        
        let mut release_data = HashMap::new();
        release_data.insert("tag_name", serde_json::Value::String(tag_name.to_string()));
        release_data.insert("name", serde_json::Value::String(name.to_string()));
        release_data.insert("draft", serde_json::Value::Bool(draft));
        release_data.insert("prerelease", serde_json::Value::Bool(prerelease));
        
        if let Some(body) = body {
            release_data.insert("body", serde_json::Value::String(body.to_string()));
        }
        
        self.make_post_request(&endpoint, &release_data).await
    }
    
    pub async fn get_commit_status(&self, owner: &str, repo: &str, sha: &str) -> Result<serde_json::Value> {
        let endpoint = format!("repos/{}/{}/commits/{}/status", owner, repo, sha);
        self.make_request(&endpoint).await
    }
    
    pub async fn list_check_runs(&self, owner: &str, repo: &str, ref_name: &str) -> Result<serde_json::Value> {
        let endpoint = format!("repos/{}/{}/commits/{}/check-runs", owner, repo, ref_name);
        self.make_request(&endpoint).await
    }
    
    pub fn extract_owner_repo_from_url(url: &str) -> Option<(String, String)> {
        if url.contains("github.com") {
            let parts: Vec<&str> = url.split('/').collect();
            if parts.len() >= 2 {
                let owner = parts[parts.len() - 2].to_string();
                let mut repo = parts[parts.len() - 1].to_string();
                
                // Remove .git suffix if present
                if repo.ends_with(".git") {
                    repo = repo[..repo.len() - 4].to_string();
                }
                
                return Some((owner, repo));
            }
        }
        None
    }
}