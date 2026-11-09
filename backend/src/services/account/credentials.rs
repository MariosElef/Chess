//! What a sign-up must look like: each check gives the value to store, or the field's error.
use crate::errors::FieldError;

const EMAIL_MAX: usize = 254;
const USERNAME_MIN: usize = 3;
const USERNAME_MAX: usize = 20;
const PASSWORD_MIN: usize = 8;
const PASSWORD_MAX: usize = 128;

/// Trimmed and lowercased, so one address is one account however it is typed.
/// No email, or a blank one, is none; any other is checked as an email.
pub fn checked_optional_email(email: Option<&str>) -> Result<Option<String>, FieldError> {
    match email.map(str::trim).filter(|email| !email.is_empty()) {
        Some(email) => checked_email(email).map(Some),
        None => Ok(None),
    }
}

pub fn checked_email(email: &str) -> Result<String, FieldError> {
    let email = email.trim().to_lowercase();
    if email.chars().count() > EMAIL_MAX || !looks_like_email(&email) {
        return Err(FieldError::new(
            "/email",
            format!("'{email}' is not an email address"),
        ));
    }
    Ok(email)
}

/// Letters, digits and underscores, as the database also checks; its case is kept.
pub fn checked_username(username: &str) -> Result<String, FieldError> {
    let username = username.trim();
    let length = username.chars().count();
    let allowed = username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !(USERNAME_MIN..=USERNAME_MAX).contains(&length) || !allowed {
        return Err(FieldError::new(
            "/username",
            format!(
                "A username is {USERNAME_MIN} to {USERNAME_MAX} letters, digits or underscores"
            ),
        ));
    }
    Ok(username.to_owned())
}

/// Kept exactly as typed: spaces are part of a password.
pub fn checked_password(password: &str) -> Result<String, FieldError> {
    let length = password.chars().count();
    if !(PASSWORD_MIN..=PASSWORD_MAX).contains(&length) {
        return Err(FieldError::new(
            "/password",
            format!("A password is {PASSWORD_MIN} to {PASSWORD_MAX} characters"),
        ));
    }
    Ok(password.to_owned())
}

/// One `@` with a name before it and a dotted domain after it, and no spaces: catches typos.
fn looks_like_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.contains('@')
        && domain.split('.').count() >= 2
        && domain.split('.').all(|part| !part.is_empty())
        && !email.contains(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::{checked_email, checked_optional_email, checked_password, checked_username};

    #[test]
    fn no_email_or_a_blank_one_is_none_and_any_other_is_checked() {
        assert_eq!(checked_optional_email(None), Ok(None));
        assert_eq!(checked_optional_email(Some("   ")), Ok(None));
        assert_eq!(
            checked_optional_email(Some("Mario@Example.com")),
            Ok(Some("mario@example.com".to_owned()))
        );
        assert!(checked_optional_email(Some("mario")).is_err());
    }

    #[test]
    fn emails_are_trimmed_and_lowercased() {
        assert_eq!(
            checked_email("  Mario@Example.COM "),
            Ok("mario@example.com".to_owned())
        );
    }

    #[test]
    fn malformed_emails_are_refused() {
        for email in [
            "mario",
            "@example.com",
            "mario@",
            "mario@example",
            "ma rio@example.com",
            "a@b@c.com",
            "mario@example..com",
        ] {
            assert!(checked_email(email).is_err(), "{email}");
        }
    }

    #[test]
    fn usernames_are_three_to_twenty_word_characters() {
        assert_eq!(checked_username("Mario_03"), Ok("Mario_03".to_owned()));
        for username in ["ab", "a_name_far_too_long_x", "mario!", "ma rio"] {
            assert!(checked_username(username).is_err(), "{username}");
        }
    }

    #[test]
    fn passwords_are_eight_to_a_hundred_and_twenty_eight_characters() {
        assert!(checked_password("seven77").is_err());
        assert!(checked_password(" eight 8").is_ok());
        assert!(checked_password(&"x".repeat(129)).is_err());
    }
}
