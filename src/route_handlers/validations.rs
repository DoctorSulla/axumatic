use super::ErrorList;

pub fn validate_email(email: &String) -> Result<bool, ErrorList> {
    if email.contains('@') && email.len() > 3 {
        return Ok(true);
    }
    Err(ErrorList::InvalidEmail)
}

pub fn validate_password(password: &String) -> Result<bool, ErrorList> {
    if password.len() < 8 && password.len() < 100 {
        return Ok(true);
    }
    Err(ErrorList::InvalidPassword)
}
