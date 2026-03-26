use crate::toys::{GithubBackend, GithubListParams, GithubToy};

pub fn list_all_issues<B: GithubBackend>(
    owner: &str,
    repo: &str,
    since: Option<String>,
    state: Option<String>,
    per_page: u32,
) -> Result<Vec<String>, String> {
    let toy = GithubToy::<B>::new();
    let mut pages = Vec::new();
    let mut page = Some(1u32);

    loop {
        let params = GithubListParams {
            since: since.clone(),
            state: state.clone(),
            page,
            per_page: Some(per_page),
        };
        let current = toy.list_issues(owner, repo, &params)?;
        pages.push(current.items);
        if !current.has_next {
            break;
        }
        page = current.next_page.or_else(|| page.map(|value| value + 1));
    }

    Ok(pages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toys::GithubPage;
    use std::sync::{Mutex, OnceLock};

    fn calls() -> &'static Mutex<Vec<GithubListParams>> {
        static CALLS: OnceLock<Mutex<Vec<GithubListParams>>> = OnceLock::new();
        CALLS.get_or_init(|| Mutex::new(Vec::new()))
    }

    struct MockGithub;

    impl GithubBackend for MockGithub {
        fn list_issues(
            _owner: &str,
            _repo: &str,
            params: &GithubListParams,
        ) -> Result<GithubPage, String> {
            calls().lock().unwrap().push(params.clone());
            match params.page.unwrap_or(1) {
                1 => Ok(GithubPage {
                    items: "page-1".to_string(),
                    has_next: true,
                    next_page: Some(2),
                    rate_remaining: 99,
                }),
                _ => Ok(GithubPage {
                    items: "page-2".to_string(),
                    has_next: false,
                    next_page: None,
                    rate_remaining: 98,
                }),
            }
        }

        fn list_pulls(
            _owner: &str,
            _repo: &str,
            _params: &GithubListParams,
        ) -> Result<GithubPage, String> {
            Err("unused in test".to_string())
        }

        fn list_issue_comments(
            _owner: &str,
            _repo: &str,
            _issue_number: u32,
        ) -> Result<GithubPage, String> {
            Err("unused in test".to_string())
        }

        fn list_issue_events(
            _owner: &str,
            _repo: &str,
            _issue_number: u32,
        ) -> Result<GithubPage, String> {
            Err("unused in test".to_string())
        }

        fn list_pull_comments(
            _owner: &str,
            _repo: &str,
            _pull_number: u32,
        ) -> Result<GithubPage, String> {
            Err("unused in test".to_string())
        }

        fn list_reviews(
            _owner: &str,
            _repo: &str,
            _pull_number: u32,
        ) -> Result<GithubPage, String> {
            Err("unused in test".to_string())
        }

        fn list_review_comments(
            _owner: &str,
            _repo: &str,
            _pull_number: u32,
            _review_id: u64,
        ) -> Result<GithubPage, String> {
            Err("unused in test".to_string())
        }
    }

    #[test]
    fn paginates_issues_with_expected_params() {
        calls().lock().unwrap().clear();
        let pages = list_all_issues::<MockGithub>(
            "patina",
            "patina",
            Some("2026-03-01T00:00:00Z".to_string()),
            Some("open".to_string()),
            50,
        )
        .unwrap();

        assert_eq!(pages, vec!["page-1".to_string(), "page-2".to_string()]);
        let captured = calls().lock().unwrap().clone();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[0].page, Some(1));
        assert_eq!(captured[1].page, Some(2));
        assert_eq!(captured[0].per_page, Some(50));
        assert_eq!(captured[0].state.as_deref(), Some("open"));
    }
}
