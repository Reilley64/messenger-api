use rspc::Error;

use crate::{dtos::UserResponseDto, AppContext};

pub async fn get_auth_user(ctx: AppContext) -> Result<UserResponseDto, Error> {
        let user = ctx.get_auth_user().await?;

        let user_response = UserResponseDto::from(user);

        Ok(user_response)
}
