use super::*;

mod json_error_format {
    use super::*;

    #[test]
    fn error_without_details() {
        let (status, Json(body)) = json_error(StatusCode::BAD_REQUEST, "Invalid input", None);
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], "Invalid input");
        assert!(body.get("details").is_none());
    }

    #[test]
    fn error_with_details() {
        let (status, Json(body)) = json_error(
            StatusCode::BAD_REQUEST,
            "Commit not on branch",
            Some("Use git rev-parse HEAD on main".to_string()),
        );
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], "Commit not on branch");
        assert_eq!(body["details"], "Use git rev-parse HEAD on main");
    }

    #[test]
    fn internal_server_error_status() {
        let (status, Json(body)) =
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error", None);
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["error"], "Database error");
    }

    #[test]
    fn not_found_error_status() {
        let (status, Json(body)) = json_error(StatusCode::NOT_FOUND, "Task not found", None);
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "Task not found");
    }
}

mod sha_validation {
    use super::*;

    #[test]
    fn valid_sha_40_lowercase_hex() {
        let sha = "a1b2c3d4e5f6789012345678901234567890abcd";
        assert!(is_valid_git_sha(sha));
    }

    #[test]
    fn valid_sha_40_uppercase_hex() {
        let sha = "A1B2C3D4E5F6789012345678901234567890ABCD";
        assert!(is_valid_git_sha(sha));
    }

    #[test]
    fn valid_sha_mixed_case() {
        let sha = "a1B2c3D4e5F6789012345678901234567890AbCd";
        assert!(is_valid_git_sha(sha));
    }

    #[test]
    fn valid_sha_all_digits() {
        let sha = "1234567890123456789012345678901234567890";
        assert!(is_valid_git_sha(sha));
    }

    #[test]
    fn invalid_sha_too_short() {
        let sha = "a1b2c3d4";
        assert!(!is_valid_git_sha(sha));
    }

    #[test]
    fn invalid_sha_too_long() {
        let sha = "a1b2c3d4e5f6789012345678901234567890abcd1234";
        assert!(!is_valid_git_sha(sha));
    }

    #[test]
    fn invalid_sha_non_hex_chars() {
        let sha = "g1b2c3d4e5f6789012345678901234567890abcd"; // 'g' is not hex
        assert!(!is_valid_git_sha(sha));
    }

    #[test]
    fn invalid_sha_empty() {
        let sha = "";
        assert!(!is_valid_git_sha(sha));
    }

    #[test]
    fn invalid_sha_spaces() {
        let sha = "a1b2c3d4e5f67890 2345678901234567890abcd";
        assert!(!is_valid_git_sha(sha));
    }

    #[test]
    fn invalid_sha_short_sha_format() {
        // Short SHA (7 chars) should be rejected
        let sha = "a1b2c3d";
        assert!(!is_valid_git_sha(sha));
    }
}
