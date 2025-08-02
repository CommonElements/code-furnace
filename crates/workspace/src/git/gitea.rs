use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaRepository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub description: String,
    pub private: bool,
    pub fork: bool,
    pub html_url: String,
    pub clone_url: String,
    pub ssh_url: String,
    pub default_branch: String,
    pub language: String,
    pub stars_count: u32,
    pub forks_count: u32,
    pub open_issues_count: u32,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaPullRequest {
    pub id: u64,
    pub number: u32,
    pub title: String,
    pub body: String,
    pub state: String,
    pub html_url: String,
    pub user: GiteaUser,
    pub head: GiteaBranch,
    pub base: GiteaBranch,
    pub created_at: String,
    pub updated_at: String,
    pub mergeable: bool,
    pub merged: bool,
    pub merged_at: Option<String>,
    pub comments: u32,
    pub additions: u32,
    pub deletions: u32,
    pub changed_files: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaIssue {
    pub id: u64,
    pub number: u32,
    pub title: String,
    pub body: String,
    pub state: String,
    pub html_url: String,
    pub user: GiteaUser,
    pub assignees: Vec<GiteaUser>,
    pub labels: Vec<GiteaLabel>,
    pub milestone: Option<GiteaMilestone>,
    pub created_at: String,
    pub updated_at: String,
    pub comments: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaUser {
    pub id: u64,
    pub login: String,
    pub full_name: String,
    pub email: String,
    pub avatar_url: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaBranch {
    pub label: String,
    pub r#ref: String,
    pub sha: String,
    pub repo: Option<GiteaRepository>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaLabel {
    pub id: u64,
    pub name: String,
    pub color: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaMilestone {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub state: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaRelease {
    pub id: u64,
    pub tag_name: String,
    pub name: String,
    pub body: String,
    pub draft: bool,
    pub prerelease: bool,
    pub html_url: String,
    pub created_at: String,
    pub published_at: String,
    pub author: GiteaUser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaCommitStatus {
    pub id: u64,
    pub status: String, // pending, success, error, failure, warning
    pub target_url: String,
    pub description: String,
    pub context: String,
    pub creator: GiteaUser,
    pub created_at: String,
    pub updated_at: String,
}

pub struct GiteaClient {
    client: Client,
    token: Option<String>,
    base_url: String,
}

impl GiteaClient {
    pub fn new(base_url: String, token: Option<String>) -> Self {
        let api_url = if base_url.ends_with("/api/v1") {
            base_url
        } else {
            format!("{}/api/v1", base_url.trim_end_matches('/'))
        };
        
        Self {
            client: Client::new(),
            token,
            base_url: api_url,
        }
    }
    
    // Convenience constructor for common Gitea instances
    pub fn gitea_com(token: Option<String>) -> Self {
        Self::new("https://gitea.com".to_string(), token)
    }
    
    pub fn codeberg(token: Option<String>) -> Self {
        Self::new("https://codeberg.org".to_string(), token)
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
            return Err(anyhow::anyhow!("Gitea API error: {}", response.status()));
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
            return Err(anyhow::anyhow!("Gitea API error: {}", response.status()));
        }
        
        let json: T = response.json().await?;
        Ok(json)
    }
    
    pub async fn get_repository(&self, owner: &str, repo: &str) -> Result<GiteaRepository> {
        let endpoint = format!("repos/{}/{}", owner, repo);
        self.make_request(&endpoint).await
    }
    
    pub async fn list_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        state: Option<&str>
    ) -> Result<Vec<GiteaPullRequest>> {
        let mut endpoint = format!("repos/{}/{}/pulls", owner, repo);
        
        if let Some(state) = state {
            endpoint.push_str(&format!("?state={}", state));
        }
        
        self.make_request(&endpoint).await
    }
    
    pub async fn get_pull_request(&self, owner: &str, repo: &str, number: u32) -> Result<GiteaPullRequest> {
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
        base: &str
    ) -> Result<GiteaPullRequest> {
        let endpoint = format!("repos/{}/{}/pulls", owner, repo);
        
        let mut pr_data = HashMap::new();
        pr_data.insert("title", serde_json::Value::String(title.to_string()));
        pr_data.insert("head", serde_json::Value::String(head.to_string()));
        pr_data.insert("base", serde_json::Value::String(base.to_string()));
        
        if let Some(body) = body {
            pr_data.insert("body", serde_json::Value::String(body.to_string()));
        }
        
        self.make_post_request(&endpoint, &pr_data).await
    }
    
    pub async fn list_issues(
        &self,
        owner: &str,
        repo: &str,
        state: Option<&str>,
        labels: Option<&str>
    ) -> Result<Vec<GiteaIssue>> {
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
    
    pub async fn get_issue(&self, owner: &str, repo: &str, number: u32) -> Result<GiteaIssue> {
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
    ) -> Result<GiteaIssue> {
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
    
    pub async fn list_releases(&self, owner: &str, repo: &str) -> Result<Vec<GiteaRelease>> {
        let endpoint = format!("repos/{}/{}/releases", owner, repo);
        self.make_request(&endpoint).await
    }
    
    pub async fn get_latest_release(&self, owner: &str, repo: &str) -> Result<GiteaRelease> {
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
    ) -> Result<GiteaRelease> {
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
    
    pub async fn get_commit_status(&self, owner: &str, repo: &str, sha: &str) -> Result<Vec<GiteaCommitStatus>> {
        let endpoint = format!("repos/{}/{}/statuses/{}", owner, repo, sha);
        self.make_request(&endpoint).await
    }
    
    pub async fn create_commit_status(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
        status: &str,
        target_url: Option<&str>,
        description: Option<&str>,
        context: Option<&str>
    ) -> Result<GiteaCommitStatus> {
        let endpoint = format!("repos/{}/{}/statuses/{}", owner, repo, sha);
        
        let mut status_data = HashMap::new();
        status_data.insert("state", serde_json::Value::String(status.to_string()));
        
        if let Some(target_url) = target_url {
            status_data.insert("target_url", serde_json::Value::String(target_url.to_string()));
        }
        
        if let Some(description) = description {
            status_data.insert("description", serde_json::Value::String(description.to_string()));
        }
        
        if let Some(context) = context {
            status_data.insert("context", serde_json::Value::String(context.to_string()));
        }
        
        self.make_post_request(&endpoint, &status_data).await
    }
    
    pub async fn get_user_repositories(&self, username: &str) -> Result<Vec<GiteaRepository>> {
        let endpoint = format!("users/{}/repos", username);
        self.make_request(&endpoint).await
    }
    
    pub async fn get_organization_repositories(&self, org: &str) -> Result<Vec<GiteaRepository>> {
        let endpoint = format!("orgs/{}/repos", org);
        self.make_request(&endpoint).await
    }
    
    pub async fn get_current_user(&self) -> Result<GiteaUser> {
        let endpoint = "user".to_string();
        self.make_request(&endpoint).await
    }
    
    pub async fn search_repositories(&self, query: &str, limit: Option<u32>) -> Result<Vec<GiteaRepository>> {
        let mut endpoint = format!("repos/search?q={}", urlencoding::encode(query));
        
        if let Some(limit) = limit {
            endpoint.push_str(&format!("&limit={}", limit));
        }
        
        #[derive(Deserialize)]
        struct SearchResponse {
            data: Vec<GiteaRepository>,
        }
        
        let response: SearchResponse = self.make_request(&endpoint).await?;
        Ok(response.data)
    }
    
    pub fn extract_owner_repo_from_url(url: &str) -> Option<(String, String)> {
        // Handle various Gitea URL formats
        if url.contains("/git/") || url.contains("gitea") || url.contains("codeberg.org") {
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
    
    pub fn detect_gitea_instance(url: &str) -> Option<String> {
        if url.contains("gitea.com") {
            Some("https://gitea.com".to_string())
        } else if url.contains("codeberg.org") {
            Some("https://codeberg.org".to_string())
        } else if url.contains("forgejo") {
            // Extract base URL for Forgejo instances
            let parts: Vec<&str> = url.split('/').collect();
            if parts.len() >= 3 {
                Some(format!("{}://{}", parts[0], parts[2]))
            } else {
                None
            }
        } else if url.contains("gitea") {
            // Try to extract base URL for self-hosted Gitea
            let parts: Vec<&str> = url.split('/').collect();
            if parts.len() >= 3 {
                Some(format!("{}://{}", parts[0], parts[2]))
            } else {
                None
            }
        } else {
            None
        }
    }
}