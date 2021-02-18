use std::env;
use std::sync::Arc;

pub struct AuthService {
    users: Arc<Vec<(String, String)>>,
}

impl AuthService {
    pub fn from_env() -> Self {
        let admin_pass = env::var("ADMIN_PASS").expect("ADMIN_PASS not set");
        let user_pass = env::var("USER_PASS").expect("USER_PASS not set");
        let users = vec![
            ("admin".to_string(), admin_pass),
            ("user".to_string(), user_pass),
        ];

        AuthService {
            users: Arc::new(users),
        }
    }

    pub fn check_credentials(&self, user_id: &str, user_pass: &str) -> bool {
        self.users
            .iter()
            .any(|(user, pass)| user == user_id && pass == user_pass)
    }
}
