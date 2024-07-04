use super::set_account_webhook::SetAccountWebhookUseCase;
use crate::error::NettuError;
use crate::shared::auth::protect_account_route;
use crate::shared::usecase::execute;
use actix_web::{web, HttpRequest, HttpResponse};
use nettu_scheduler_api_structs::delete_account_webhook::APIResponse;
use nettu_scheduler_infra::NettuContext;

pub async fn delete_account_webhook_controller(
    http_req: HttpRequest,
    ctx: web::Data<NettuContext>,
) -> Result<HttpResponse, NettuError> {
    let account = protect_account_route(&http_req, &ctx).await?;

    let usecase = SetAccountWebhookUseCase {
        account,
        webhook_url: None,
    };

    execute(usecase, &ctx)
        .await
        .map(|account| HttpResponse::Ok().json(APIResponse::new(account)))
        .map_err(NettuError::from)
}
