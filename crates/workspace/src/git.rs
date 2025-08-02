use anyhow::Result;
use git2::{Repository, StatusOptions, Signature, DiffOptions, Branch, BranchType};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashSet;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRepository {
    pub path: PathBuf,
    pub branch: String,
    pub status: GitStatus,
    pub remote_url: Option<String>,
    pub platform: GitPlatform,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub staged: Vec<GitFileStatus>,
    pub unstaged: Vec<GitFileStatus>,
    pub untracked: Vec<String>,
    pub ahead: usize,
    pub behind: usize,
    pub conflicts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitFileStatus {
    pub path: String,
    pub status: GitFileChangeType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GitFileChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Conflicted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommit {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: GitAuthor,
    pub committer: GitAuthor,
    pub timestamp: DateTime<Utc>,
    pub parents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitAuthor {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBranch {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiff {
    pub old_file: Option<String>,
    pub new_file: Option<String>,
    pub hunks: Vec<GitDiffHunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<GitDiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffLine {
    pub line_type: GitDiffLineType,
    pub content: String,
    pub old_line_no: Option<u32>,
    pub new_line_no: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GitDiffLineType {
    Context,
    Addition,
    Deletion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GitPlatform {
    GitHub,
    GitLab,
    Gitea,
    Forgejo,
    Bitbucket,
    Generic,
}

impl GitPlatform {
    pub fn detect_from_url(url: &str) -> Self {
        if url.contains("github.com") {
            GitPlatform::GitHub
        } else if url.contains("gitlab.com") || url.contains("gitlab.") {
            GitPlatform::GitLab
        } else if url.contains("gitea.") || url.contains("/gitea/") {
            GitPlatform::Gitea
        } else if url.contains("forgejo.") || url.contains("/forgejo/") {
            GitPlatform::Forgejo
        } else if url.contains("bitbucket.org") {
            GitPlatform::Bitbucket
        } else {
            GitPlatform::Generic
        }
    }
}

pub struct GitManager {
    // Track which repositories we've seen, but don't store Repository objects (they're not Send + Sync)
    pub known_repositories: HashSet<PathBuf>,
}

impl GitManager {
    pub fn new() -> Self {
        Self {
            known_repositories: HashSet::new(),
        }
    }
    
    pub fn open_repository(&mut self, path: PathBuf) -> Result<GitRepository> {
        let repo = Repository::open(&path)?;
        
        // Get current branch
        let branch = {
            let head = repo.head()?;
            if head.is_branch() {
                head.shorthand().unwrap_or("unknown").to_string()
            } else {
                "detached".to_string()
            }
        };
        
        // Get remote URL
        let remote_url = repo.find_remote("origin")
            .ok()
            .and_then(|remote| remote.url().map(|s| s.to_string()));
        
        // Detect platform
        let platform = remote_url.as_ref()
            .map(|url| GitPlatform::detect_from_url(url))
            .unwrap_or(GitPlatform::Generic);
        
        // Get status
        let status = self.get_status(&repo)?;
        
        let git_repo = GitRepository {
            path: path.clone(),
            branch,
            status,
            remote_url,
            platform,
        };
        
        // Track this repository
        self.known_repositories.insert(path);
        Ok(git_repo)
    }
    
    pub fn get_status(&self, repo: &Repository) -> Result<GitStatus> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true);
        opts.include_ignored(false);
        
        let statuses = repo.statuses(Some(&mut opts))?;
        
        let mut staged = Vec::new();
        let mut unstaged = Vec::new();
        let mut untracked = Vec::new();
        let mut conflicts = Vec::new();
        
        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("unknown").to_string();
            let status = entry.status();
            
            if status.is_conflicted() {
                conflicts.push(path.clone());
            }
            
            if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() || status.is_index_renamed() {
                let change_type = if status.is_index_new() {
                    GitFileChangeType::Added
                } else if status.is_index_modified() {
                    GitFileChangeType::Modified
                } else if status.is_index_deleted() {
                    GitFileChangeType::Deleted
                } else if status.is_index_renamed() {
                    GitFileChangeType::Renamed
                } else {
                    GitFileChangeType::Modified
                };
                
                staged.push(GitFileStatus {
                    path: path.clone(),
                    status: change_type,
                });
            }
            
            if status.is_wt_new() {
                untracked.push(path.clone());
            } else if status.is_wt_modified() || status.is_wt_deleted() || status.is_wt_renamed() {
                let change_type = if status.is_wt_modified() {
                    GitFileChangeType::Modified
                } else if status.is_wt_deleted() {
                    GitFileChangeType::Deleted
                } else if status.is_wt_renamed() {
                    GitFileChangeType::Renamed
                } else {
                    GitFileChangeType::Modified
                };
                
                unstaged.push(GitFileStatus {
                    path: path.clone(),
                    status: change_type,
                });
            }
        }
        
        // Get ahead/behind count
        let (ahead, behind) = self.get_ahead_behind_count(repo).unwrap_or((0, 0));
        
        Ok(GitStatus {
            staged,
            unstaged,
            untracked,
            ahead,
            behind,
            conflicts,
        })
    }
    
    fn get_ahead_behind_count(&self, repo: &Repository) -> Result<(usize, usize)> {
        let head = repo.head()?;
        let head_oid = head.target().ok_or_else(|| anyhow::anyhow!("No HEAD commit"))?;
        
        // Find upstream branch
        let branch = Branch::wrap(head);
        let upstream = branch.upstream()?;
        let upstream_oid = upstream.get().target().ok_or_else(|| anyhow::anyhow!("No upstream commit"))?;
        
        let (ahead, behind) = repo.graph_ahead_behind(head_oid, upstream_oid)?;
        Ok((ahead, behind))
    }
    
    pub fn stage_file(&mut self, repo_path: &PathBuf, file_path: &str) -> Result<()> {
        let repo = Repository::open(repo_path)?;
        
        let mut index = repo.index()?;
        index.add_path(std::path::Path::new(file_path))?;
        index.write()?;
        
        Ok(())
    }
    
    pub fn unstage_file(&mut self, repo_path: &PathBuf, file_path: &str) -> Result<()> {
        let repo = Repository::open(repo_path)?;
        
        let head = repo.head()?;
        let head_tree = head.peel_to_tree()?;
        
        let mut checkout_builder = git2::build::CheckoutBuilder::new();
        checkout_builder.path(file_path);
        checkout_builder.force();
        
        repo.checkout_tree(head_tree.as_object(), Some(&mut checkout_builder))?;
        
        Ok(())
    }
    
    pub fn commit(&mut self, repo_path: &PathBuf, message: &str, author_name: &str, author_email: &str) -> Result<String> {
        let repo = Repository::open(repo_path)?;
        
        let signature = Signature::now(author_name, author_email)?;
        let mut index = repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        
        let head = repo.head()?;
        let parent_commit = head.peel_to_commit()?;
        
        let commit_id = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent_commit],
        )?;
        
        Ok(commit_id.to_string())
    }
    
    pub fn get_commit_history(&self, repo_path: &PathBuf, limit: Option<usize>) -> Result<Vec<GitCommit>> {
        let repo = Repository::open(repo_path)?;
        
        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME)?;
        
        let mut commits = Vec::new();
        let max_commits = limit.unwrap_or(100);
        
        for (i, commit_id) in revwalk.enumerate() {
            if i >= max_commits {
                break;
            }
            
            let commit_id = commit_id?;
            let commit = repo.find_commit(commit_id)?;
            
            let author = commit.author();
            let committer = commit.committer();
            
            let git_commit = GitCommit {
                hash: commit_id.to_string(),
                short_hash: format!("{:.7}", commit_id.to_string()),
                message: commit.message().unwrap_or("").to_string(),
                author: GitAuthor {
                    name: author.name().unwrap_or("").to_string(),
                    email: author.email().unwrap_or("").to_string(),
                },
                committer: GitAuthor {
                    name: committer.name().unwrap_or("").to_string(),
                    email: committer.email().unwrap_or("").to_string(),
                },
                timestamp: DateTime::from_timestamp(commit.time().seconds(), 0)
                    .unwrap_or_else(|| Utc::now()),
                parents: commit.parent_ids().map(|id| id.to_string()).collect(),
            };
            
            commits.push(git_commit);
        }
        
        Ok(commits)
    }
    
    pub fn get_branches(&self, repo_path: &PathBuf) -> Result<Vec<GitBranch>> {
        let repo = Repository::open(repo_path)?;
        
        let mut branches = Vec::new();
        let current_branch = repo.head()?.shorthand().unwrap_or("").to_string();
        
        // Local branches
        let local_branches = repo.branches(Some(BranchType::Local))?;
        for branch in local_branches {
            let (branch, _) = branch?;
            let name = branch.name()?.unwrap_or("").to_string();
            let is_current = name == current_branch;
            
            let upstream = branch.upstream().ok()
                .and_then(|upstream| upstream.name().ok().flatten().map(|s| s.to_string()));
            
            // Calculate ahead/behind for this branch
            let (ahead, behind) = if let (Some(local_commit), Some(upstream_name)) = (branch.get().target(), &upstream) {
                if let Ok(upstream_ref) = repo.find_reference(upstream_name) {
                    if let Some(upstream_commit) = upstream_ref.target() {
                        repo.graph_ahead_behind(local_commit, upstream_commit).unwrap_or((0, 0))
                    } else {
                        (0, 0)
                    }
                } else {
                    (0, 0)
                }
            } else {
                (0, 0)
            };
            
            branches.push(GitBranch {
                name,
                is_current,
                is_remote: false,
                upstream,
                ahead,
                behind,
            });
        }
        
        // Remote branches
        let remote_branches = repo.branches(Some(BranchType::Remote))?;
        for branch in remote_branches {
            let (branch, _) = branch?;
            let name = branch.name()?.unwrap_or("").to_string();
            
            branches.push(GitBranch {
                name,
                is_current: false,
                is_remote: true,
                upstream: None,
                ahead: 0,
                behind: 0,
            });
        }
        
        Ok(branches)
    }
    
    pub fn create_branch(&mut self, repo_path: &PathBuf, branch_name: &str, from_head: bool) -> Result<()> {
        let repo = Repository::open(repo_path)?;
        
        let target_commit = if from_head {
            repo.head()?.peel_to_commit()?
        } else {
            return Err(anyhow::anyhow!("Creating branch from specific commit not yet implemented"));
        };
        
        repo.branch(branch_name, &target_commit, false)?;
        Ok(())
    }
    
    pub fn switch_branch(&mut self, repo_path: &PathBuf, branch_name: &str) -> Result<()> {
        let repo = Repository::open(repo_path)?;
        
        let branch_ref = format!("refs/heads/{}", branch_name);
        let reference = repo.find_reference(&branch_ref)?;
        let commit = reference.peel_to_commit()?;
        
        // Checkout the branch
        let mut checkout_builder = git2::build::CheckoutBuilder::new();
        checkout_builder.safe();
        
        repo.checkout_tree(commit.as_object(), Some(&mut checkout_builder))?;
        repo.set_head(&branch_ref)?;
        
        Ok(())
    }
    
    pub fn get_diff(&self, repo_path: &PathBuf, staged: bool) -> Result<Vec<GitDiff>> {
        let repo = Repository::open(repo_path)?;
        
        let mut diff_opts = DiffOptions::new();
        diff_opts.context_lines(3);
        
        let diff = if staged {
            // Diff between HEAD and index (staged changes)
            let head_tree = repo.head()?.peel_to_tree()?;
            repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut diff_opts))?
        } else {
            // Diff between index and working directory (unstaged changes)
            repo.diff_index_to_workdir(None, Some(&mut diff_opts))?
        };
        
        let mut git_diffs = Vec::new();
        
        // Use print() to get the raw diff text for now
        // This is a simpler approach that avoids the complex foreach callback issues
        let mut diff_text = Vec::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            diff_text.extend_from_slice(line.content());
            true
        })?;
        
        // For now, return a simplified diff structure
        // In a full implementation, you'd parse the diff text to create proper GitDiff structures
        let diff_string = String::from_utf8_lossy(&diff_text);
        if !diff_string.is_empty() {
            git_diffs.push(GitDiff {
                old_file: Some("modified files".to_string()),
                new_file: Some("modified files".to_string()),
                hunks: vec![GitDiffHunk {
                    old_start: 0,
                    old_lines: 0,
                    new_start: 0,
                    new_lines: 0,
                    header: "Diff".to_string(),
                    lines: vec![GitDiffLine {
                        line_type: GitDiffLineType::Context,
                        content: diff_string.to_string(),
                        old_line_no: None,
                        new_line_no: None,
                    }],
                }],
            });
        }
        
        Ok(git_diffs)
    }
    
    pub fn push(&self, repo_path: &PathBuf, remote: &str, branch: &str) -> Result<()> {
        let repo = Repository::open(repo_path)?;
        
        let mut remote = repo.find_remote(remote)?;
        let refspec = format!("refs/heads/{}:refs/heads/{}", branch, branch);
        
        // Note: This is a simplified push without authentication
        // In a real implementation, you'd need to handle credentials
        remote.push(&[&refspec], None)?;
        
        Ok(())
    }
    
    pub fn pull(&self, repo_path: &PathBuf, remote: &str, branch: &str) -> Result<()> {
        let repo = Repository::open(repo_path)?;
        
        // This is a simplified pull implementation
        // In practice, you'd want to handle merge conflicts, fast-forward only, etc.
        let mut remote = repo.find_remote(remote)?;
        remote.fetch(&[branch], None, None)?;
        
        // Merge FETCH_HEAD
        let fetch_head = repo.find_reference("FETCH_HEAD")?;
        let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
        
        // Perform the merge
        let analysis = repo.merge_analysis(&[&fetch_commit])?;
        
        if analysis.0.is_fast_forward() {
            // Fast-forward merge
            let refname = format!("refs/heads/{}", branch);
            let mut reference = repo.find_reference(&refname)?;
            reference.set_target(fetch_commit.id(), "Fast-forward")?;
            repo.set_head(&refname)?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        } else if analysis.0.is_normal() {
            // Normal merge (would need conflict resolution)
            return Err(anyhow::anyhow!("Normal merge not implemented - conflicts may exist"));
        }
        
        Ok(())
    }
    
    pub fn generate_ai_commit_message(&self, _repo_path: &PathBuf, staged_files: &[String]) -> Result<String> {
        // This would integrate with the AI agent system
        // For now, return a placeholder
        let file_list = if staged_files.is_empty() {
            "multiple files".to_string()
        } else if staged_files.len() == 1 {
            staged_files[0].clone()
        } else {
            format!("{} files", staged_files.len())
        };
        
        // TODO: Integrate with AI agent to analyze changes and generate intelligent commit message
        Ok(format!("Update {}", file_list))
    }
}

// Git platform API integrations
pub mod github;
pub mod gitlab;
pub mod gitea;

pub use github::GitHubClient;
pub use gitlab::GitLabClient;  
pub use gitea::GiteaClient;