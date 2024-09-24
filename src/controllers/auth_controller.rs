use rspc::Error;

use crate::{dtos::UserResponseDto, RequestContext};

pub async fn get_auth_user(ctx: RequestContext) -> Result<UserResponseDto, Error> {
        let user = ctx.get_auth_user().await?;

        let user_response = UserResponseDto::from(user);

        Ok(user_response)
}
