use clerk_rs::validators::authorizer::ClerkJwt;
use thiserror::Error;

use crate::AppState;

#[derive(Debug, Error)]
pub enum ClerkError {
    #[error("User is missing required field: `id`")]
    MissingId,

    #[error("User (ID: {0}) is missing required field: `email_address`")]
    MissingEmailAddress(String),
}

#[derive(Debug)]
pub struct ClerkUser {
    id: String,
    email: String,
    has_image: bool,
    image_url: Option<String>,
}

impl ClerkUser {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn has_image(&self) -> bool {
        self.has_image
    }

    pub fn image_url(&self) -> Option<&str> {
        self.image_url.as_ref().map(AsRef::as_ref)
    }
}

impl TryFrom<clerk_rs::models::User> for ClerkUser {
    type Error = ClerkError;

    fn try_from(user: clerk_rs::models::User) -> Result<Self, Self::Error> {
        let id = user.id.ok_or(ClerkError::MissingId)?;

        // clerk stores user emails in a list, and has a `primary_email_address_id` field that
        // tells us which email to use.
        let email_id = user
            .primary_email_address_id
            .flatten()
            .ok_or(ClerkError::MissingEmailAddress(id.clone()))?;

        let email = user
            .email_addresses
            .ok_or(ClerkError::MissingEmailAddress(id.clone()))?
            .into_iter()
            // Safety: we checked for `primary_email_address_id` above, if it exists then email is
            // guaranteed to exist.
            .find(|email| email.id.as_ref().unwrap() == &email_id)
            .ok_or(ClerkError::MissingEmailAddress(id.clone()))?
            .email_address;

        let has_image = user.has_image.unwrap_or_default();
        let image_url = user.image_url;

        Ok(Self {
            id,
            email,
            image_url,
            has_image,
        })
    }
}

#[tracing::instrument(skip_all)]
pub async fn get_clerk_user(state: &AppState, jwt: ClerkJwt) -> anyhow::Result<ClerkUser> {
    // ID of the current user of the session (subject)
    // see https://clerk.com/docs/backend-requests/resources/session-tokens
    let user_id = jwt.sub;

    let user = clerk_rs::apis::users_api::User::get_user(&state.clerk, &user_id).await?;

    let user = ClerkUser::try_from(user)?;
    Ok(user)
}
