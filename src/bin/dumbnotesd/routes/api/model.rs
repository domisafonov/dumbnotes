use dumbnotes::username_string::UsernameStr;

struct LoginRequest<'a> {
    username: &'a UsernameStr,
    secret: LoginRequestSecret,
}

enum LoginRequestSecret {
    Password(String),
    RefreshToken(Vec<u8>),
}

struct LoginResponse {
    refresh_token: Vec<u8>,
    token: Vec<u8>,
}
