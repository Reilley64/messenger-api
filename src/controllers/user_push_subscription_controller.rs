use chrono::Utc;
use rspc::Error;

use crate::{
        dtos::{UserPushSubscriptionRequestDto, UserPushSubscriptionResponseDto},
        models::UserPushSubscription,
        RequestContext,
};

pub async fn create_user_push_subscripition(
        ctx: RequestContext,
        user_push_subscription_request: UserPushSubscriptionRequestDto,
) -> Result<UserPushSubscriptionResponseDto, Error> {
        let auth_user = ctx.get_auth_user().await?;

        let user_push_subscrption = {
                let mut id_generator = ctx.app_state.id_generator.lock().unwrap();
                ctx.app_state
                        .user_push_subscription_repository
                        .save(UserPushSubscription {
                                id: id_generator.generate(),
                                created_at: Utc::now().naive_utc(),
                                updated_at: Utc::now().naive_utc(),
                                user_id: auth_user.id,
                                endpoint: user_push_subscription_request.endpoint.clone(),
                                p256dh: user_push_subscription_request.p256dh.clone(),
                                auth: user_push_subscription_request.auth.clone(),
                        })?
        };

        let user_push_subscription_response = UserPushSubscriptionResponseDto {
                id: user_push_subscrption.id.to_string(),
                created_at: user_push_subscrption.created_at,
                updated_at: user_push_subscrption.updated_at,
                user_id: user_push_subscrption.user_id.to_string(),
                endpoint: user_push_subscrption.endpoint,
                p256dh: user_push_subscrption.p256dh,
                auth: user_push_subscrption.auth,
        };

        Ok(user_push_subscription_response)
}
