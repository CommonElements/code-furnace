use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabProject {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub path_with_namespace: String,
    pub description: Option<String>,
    pub visibility: String,
    pub web_url: String,
    pub http_url_to_repo: String,
    pub ssh_url_to_repo: String,
    pub default_branch: String,
    pub tag_list: Vec<String>,
    pub star_count: u32,
    pub forks_count: u32,
    pub open_issues_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabMergeRequest {
    pub id: u64,
    pub iid: u32,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub draft: bool,
    pub web_url: String,
    pub author: GitLabUser,
    pub assignees: Vec<GitLabUser>,
    pub reviewers: Vec<GitLabUser>,
    pub source_branch: String,
    pub target_branch: String,
    pub created_at: String,
    pub updated_at: String,
    pub merged_at: Option<String>,
    pub merge_status: String,
    pub user_notes_count: u32,
    pub upvotes: u32,
    pub downvotes: u32,
    pub changes_count: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabIssue {
    pub id: u64,
    pub iid: u32,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub web_url: String,
    pub author: GitLabUser,
    pub assignees: Vec<GitLabUser>,
    pub labels: Vec<String>,
    pub milestone: Option<GitLabMilestone>,
    pub created_at: String,
    pub updated_at: String,
    pub user_notes_count: u32,
    pub upvotes: u32,
    pub downvotes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabUser {
    pub id: u64,
    pub name: String,
    pub username: String,
    pub avatar_url: String,
    pub web_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabMilestone {
    pub id: u64,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub web_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabPipeline {
    pub id: u64,
    pub status: String,
    pub r#ref: String,
    pub sha: String,
    pub web_url: String,
    pub created_at: String,
    pub updated_at: String,
    pub user: GitLabUser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabJob {
    pub id: u64,
    pub name: String,
    pub stage: String,
    pub status: String,
    pub web_url: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration: Option<f64>,
    pub pipeline: GitLabPipelineRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabPipelineRef {
    pub id: u64,
    pub status: String,
    pub r#ref: String,
    pub sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabRelease {
    pub tag_name: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub released_at: Option<String>,
    pub author: GitLabUser,
    #[serde(rename = "_links")]
    pub links: GitLabReleaseLinks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabReleaseLinks {
    #[serde(rename = "self")]
    pub self_link: String,
    pub edit_url: String,
}

pub struct GitLabClient {
    client: Client,
    token: Option<String>,
    base_url: String,
}

impl GitLabClient {
    pub fn new(token: Option<String>) -> Self {
        Self {
            client: Client::new(),
            token,
            base_url: "https://gitlab.com/api/v4".to_string(),
        }
    }
    
    pub fn with_custom_url(token: Option<String>, base_url: String) -> Self {
        let api_url = if base_url.ends_with("/api/v4") {
            base_url
        } else {
            format!("{}/api/v4", base_url.trim_end_matches('/'))
        };
        
        Self {
            client: Client::new(),
            token,
            base_url: api_url,
        }
    }
    
    async fn make_request<T>(&self, endpoint: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{}/{}", self.base_url, endpoint);
        let mut request = self.client.get(&url);
        
        if let Some(token) = &self.token {
            request = request.header("Private-Token", token);
        }
        
        request = request.header("User-Agent", "Code-Furnace/1.0");
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitLab API error: {}", response.status()));
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
            request = request.header("Private-Token", token);
        }
        
        request = request
            .header("User-Agent", "Code-Furnace/1.0")
            .header("Content-Type", "application/json")
            .json(body);
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitLab API error: {}", response.status()));
        }
        
        let json: T = response.json().await?;
        Ok(json)
    }
    
    pub async fn get_project(&self, project_id: &str) -> Result<GitLabProject> {
        let endpoint = format!("projects/{}", urlencoding::encode(project_id));
        self.make_request(&endpoint).await
    }
    
    pub async fn list_merge_requests(
        &self,
        project_id: &str,
        state: Option<&str>,
        target_branch: Option<&str>
    ) -> Result<Vec<GitLabMergeRequest>> {
        let mut endpoint = format!("projects/{}/merge_requests", urlencoding::encode(project_id));
        let mut params = Vec::new();
        
        if let Some(state) = state {
            params.push(format!("state={}", state));
        }
        
        if let Some(target_branch) = target_branch {
            params.push(format!("target_branch={}", target_branch));
        }
        
        if !params.is_empty() {
            endpoint.push('?');
            endpoint.push_str(&params.join("&"));
        }
        
        self.make_request(&endpoint).await
    }
    
    pub async fn get_merge_request(&self, project_id: &str, mr_iid: u32) -> Result<GitLabMergeRequest> {
        let endpoint = format!("projects/{}/merge_requests/{}", urlencoding::encode(project_id), mr_iid);
        self.make_request(&endpoint).await
    }
    
    pub async fn create_merge_request(
        &self,
        project_id: &str,
        source_branch: &str,
        target_branch: &str,
        title: &str,
        description: Option<&str>,
        draft: bool
    ) -> Result<GitLabMergeRequest> {
        let endpoint = format!("projects/{}/merge_requests", urlencoding::encode(project_id));
        
        let mut mr_data = HashMap::new();
        mr_data.insert("source_branch", serde_json::Value::String(source_branch.to_string()));
        mr_data.insert("target_branch", serde_json::Value::String(target_branch.to_string()));
        mr_data.insert("title", serde_json::Value::String(title.to_string()));
        
        if let Some(description) = description {
            mr_data.insert("description", serde_json::Value::String(description.to_string()));
        }
        
        if draft {
            // GitLab uses "Draft:" prefix in title for draft MRs
            let draft_title = if title.starts_with("Draft:") || title.starts_with("WIP:") {
                title.to_string()
            } else {
                format!("Draft: {}", title)
            };
            mr_data.insert("title", serde_json::Value::String(draft_title));
        }
        
        self.make_post_request(&endpoint, &mr_data).await
    }
    
    pub async fn list_issues(
        &self,
        project_id: &str,
        state: Option<&str>,
        labels: Option<&str>
    ) -> Result<Vec<GitLabIssue>> {
        let mut endpoint = format!("projects/{}/issues", urlencoding::encode(project_id));
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
    
    pub async fn get_issue(&self, project_id: &str, issue_iid: u32) -> Result<GitLabIssue> {
        let endpoint = format!("projects/{}/issues/{}", urlencoding::encode(project_id), issue_iid);
        self.make_request(&endpoint).await
    }
    
    pub async fn create_issue(
        &self,
        project_id: &str,
        title: &str,
        description: Option<&str>,
        labels: Option<Vec<&str>>,
        assignee_ids: Option<Vec<u64>>
    ) -> Result<GitLabIssue> {
        let endpoint = format!("projects/{}/issues", urlencoding::encode(project_id));
        
        let mut issue_data = HashMap::new();
        issue_data.insert("title", serde_json::Value::String(title.to_string()));
        
        if let Some(description) = description {
            issue_data.insert("description", serde_json::Value::String(description.to_string()));
        }
        
        if let Some(labels) = labels {
            issue_data.insert("labels", serde_json::Value::String(labels.join(",")));
        }
        
        if let Some(assignee_ids) = assignee_ids {
            issue_data.insert("assignee_ids", serde_json::Value::Array(
                assignee_ids.into_iter().map(|id| serde_json::Value::Number(id.into())).collect()
            ));
        }
        
        self.make_post_request(&endpoint, &issue_data).await
    }
    
    pub async fn list_pipelines(&self, project_id: &str, ref_name: Option<&str>) -> Result<Vec<GitLabPipeline>> {
        let mut endpoint = format!("projects/{}/pipelines", urlencoding::encode(project_id));
        
        if let Some(ref_name) = ref_name {
            endpoint.push_str(&format!("?ref={}", ref_name));
        }
        
        self.make_request(&endpoint).await
    }
    
    pub async fn get_pipeline(&self, project_id: &str, pipeline_id: u64) -> Result<GitLabPipeline> {
        let endpoint = format!("projects/{}/pipelines/{}", urlencoding::encode(project_id), pipeline_id);
        self.make_request(&endpoint).await
    }
    
    pub async fn list_pipeline_jobs(&self, project_id: &str, pipeline_id: u64) -> Result<Vec<GitLabJob>> {
        let endpoint = format!("projects/{}/pipelines/{}/jobs", urlencoding::encode(project_id), pipeline_id);
        self.make_request(&endpoint).await
    }
    
    pub async fn get_job(&self, project_id: &str, job_id: u64) -> Result<GitLabJob> {
        let endpoint = format!("projects/{}/jobs/{}", urlencoding::encode(project_id), job_id);
        self.make_request(&endpoint).await
    }
    
    pub async fn list_releases(&self, project_id: &str) -> Result<Vec<GitLabRelease>> {
        let endpoint = format!("projects/{}/releases", urlencoding::encode(project_id));
        self.make_request(&endpoint).await
    }
    
    pub async fn get_release(&self, project_id: &str, tag_name: &str) -> Result<GitLabRelease> {
        let endpoint = format!("projects/{}/releases/{}", urlencoding::encode(project_id), urlencoding::encode(tag_name));
        self.make_request(&endpoint).await
    }
    
    pub async fn create_release(
        &self,
        project_id: &str,
        tag_name: &str,
        name: &str,
        description: Option<&str>
    ) -> Result<GitLabRelease> {
        let endpoint = format!("projects/{}/releases", urlencoding::encode(project_id));
        
        let mut release_data = HashMap::new();
        release_data.insert("tag_name", serde_json::Value::String(tag_name.to_string()));
        release_data.insert("name", serde_json::Value::String(name.to_string()));
        
        if let Some(description) = description {
            release_data.insert("description", serde_json::Value::String(description.to_string()));
        }
        
        self.make_post_request(&endpoint, &release_data).await
    }
    
    pub async fn get_commit_status(&self, project_id: &str, sha: &str) -> Result<serde_json::Value> {
        let endpoint = format!("projects/{}/repository/commits/{}/statuses", urlencoding::encode(project_id), sha);
        self.make_request(&endpoint).await
    }
    
    pub fn extract_project_id_from_url(url: &str) -> Option<String> {
        if url.contains("gitlab.com") || url.contains("gitlab.") {
            let parts: Vec<&str> = url.split('/').collect();
            if parts.len() >= 2 {
                let owner = parts[parts.len() - 2];
                let mut repo = parts[parts.len() - 1];
                
                // Remove .git suffix if present
                if repo.ends_with(".git") {
                    repo = &repo[..repo.len() - 4];
                }
                
                return Some(format!("{}/{}", owner, repo));
            }
        }
        None
    }
}