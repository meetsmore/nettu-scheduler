use crate::error::NettuError;
use crate::shared::{
    auth::protect_account_route,
    usecase::{execute, UseCase},
};
use actix_web::{web, HttpRequest, HttpResponse};
use nettu_scheduler_api_structs::remove_service_event_intend::*;
use nettu_scheduler_domain::{Account, ID};
use nettu_scheduler_infra::NettuContext;

pub async fn remove_service_event_intend_controller(
    http_req: HttpRequest,
    query_params: web::Query<QueryParams>,
    path_params: web::Path<PathParams>,
    ctx: web::Data<NettuContext>,
) -> Result<HttpResponse, NettuError> {
    let account = protect_account_route(&http_req, &ctx).await?;

    let query = query_params.0;
    let usecase = RemoveServiceEventIntendUseCase {
        account,
        service_id: path_params.service_id.to_owned(),
        timestamp: query.timestamp,
    };

    execute(usecase, &ctx)
        .await
        .map(|_| HttpResponse::Ok().json(APIResponse::new()))
        .map_err(|e| match e {
            UseCaseErrors::ServiceNotFound => {
                NettuError::NotFound("The requested service was not found".into())
            }
            UseCaseErrors::StorageError => NettuError::InternalError,
        })
}

#[derive(Debug)]
struct RemoveServiceEventIntendUseCase {
    pub account: Account,
    pub service_id: ID,
    pub timestamp: i64,
}

#[derive(Debug)]
struct UseCaseRes {}

#[derive(Debug)]
enum UseCaseErrors {
    ServiceNotFound,
    StorageError,
}

#[async_trait::async_trait(?Send)]
impl UseCase for RemoveServiceEventIntendUseCase {
    type Response = UseCaseRes;

    type Errors = UseCaseErrors;

    const NAME: &'static str = "RemoveServiceEventIntend";

    async fn execute(&mut self, ctx: &NettuContext) -> Result<Self::Response, Self::Errors> {
        match ctx.repos.services.find(&self.service_id).await {
            Some(s) if s.account_id == self.account.id => (),
            _ => return Err(UseCaseErrors::ServiceNotFound),
        };
        ctx.repos
            .reservations
            .remove_one(&self.service_id, self.timestamp)
            .await
            .map(|_| UseCaseRes {})
            .map_err(|_| UseCaseErrors::StorageError)
    }
}