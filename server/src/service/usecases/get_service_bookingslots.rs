use crate::{
    api::{Context, NettuError},
    calendar::usecases::get_user_freebusy::GetUserFreeBusyUseCase,
    event::domain::booking_slots::{
        get_service_bookingslots, validate_bookingslots_query, validate_slots_interval,
        BookingQueryError, BookingSlotsOptions, BookingSlotsQuery, ServiceBookingSlot,
        ServiceBookingSlotDTO,
    },
    shared::auth::ensure_nettu_acct_header,
};
use crate::{
    event::domain::booking_slots::UserFreeEvents,
    shared::usecase::{execute, Usecase},
};
use actix_web::{web, HttpRequest, HttpResponse};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PathParams {
    service_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryParams {
    iana_tz: Option<String>,
    duration: i64,
    interval: i64,
    date: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct APIRes {
    booking_slots: Vec<ServiceBookingSlotDTO>,
}

pub async fn get_service_bookingslots_controller(
    http_req: HttpRequest,
    query_params: web::Query<QueryParams>,
    path_params: web::Path<PathParams>,
    ctx: web::Data<Context>,
) -> Result<HttpResponse, NettuError> {
    let _account = ensure_nettu_acct_header(&http_req)?;

    let usecase = GetServiceBookingSlotsUseCase {
        service_id: path_params.service_id.clone(),
        iana_tz: query_params.iana_tz.clone(),
        date: query_params.date.clone(),
        duration: query_params.duration,
        interval: query_params.interval,
    };

    execute(usecase, &ctx).await
        .map(|usecase_res| {
            let res = APIRes {
                booking_slots: usecase_res
                    .booking_slots
                    .iter()
                    .map(|slot| ServiceBookingSlotDTO::new(slot))
                    .collect(),
            };
            HttpResponse::Ok().json(res)
        })
        .map_err(|e| match e {
            UseCaseErrors::InvalidDateError(msg) => {
                NettuError::BadClientData(format!(
                    "Invalid datetime: {}. Should be YYYY-MM-DD, e.g. January 1. 2020 => 2020-1-1",
                    msg
                ))
            }
            UseCaseErrors::InvalidTimezoneError(msg) => {
                NettuError::BadClientData(format!(
                    "Invalid timezone: {}. It should be a valid IANA TimeZone.",
                    msg
                ))
            }
            UseCaseErrors::InvalidIntervalError => {
                NettuError::BadClientData(
                    "Invalid interval specified. It should be between 10 - 60 minutes inclusively and be specified as milliseconds.".into()
                )
            }
            UseCaseErrors::ServiceNotFoundError => NettuError::NotFound(format!("Service with id: {}, was not found.", path_params.service_id)),
        })
}

struct GetServiceBookingSlotsUseCase {
    pub service_id: String,
    pub date: String,
    pub iana_tz: Option<String>,
    pub duration: i64,
    pub interval: i64,
}

struct UseCaseRes {
    booking_slots: Vec<ServiceBookingSlot>,
}

#[derive(Debug)]
enum UseCaseErrors {
    ServiceNotFoundError,
    InvalidIntervalError,
    InvalidDateError(String),
    InvalidTimezoneError(String),
}

#[async_trait::async_trait(?Send)]
impl Usecase for GetServiceBookingSlotsUseCase {
    type Response = UseCaseRes;

    type Errors = UseCaseErrors;

    type Context = Context;

    async fn execute(&mut self, ctx: &Self::Context) -> Result<Self::Response, Self::Errors> {
        if !validate_slots_interval(self.interval) {
            return Err(UseCaseErrors::InvalidIntervalError);
        }

        let query = BookingSlotsQuery {
            date: self.date.clone(),
            iana_tz: self.iana_tz.clone(),
            interval: self.interval,
            duration: self.duration,
        };
        let booking_timespan = match validate_bookingslots_query(&query) {
            Ok(t) => t,
            Err(e) => match e {
                BookingQueryError::InvalidIntervalError => {
                    return Err(UseCaseErrors::InvalidIntervalError)
                }
                BookingQueryError::InvalidDateError(d) => {
                    return Err(UseCaseErrors::InvalidDateError(d))
                }
                BookingQueryError::InvalidTimezoneError(d) => {
                    return Err(UseCaseErrors::InvalidTimezoneError(d))
                }
            },
        };

        let service = match ctx.repos.service_repo.find(&self.service_id).await {
            Some(s) => s,
            None => return Err(UseCaseErrors::ServiceNotFoundError),
        };

        let mut users_freebusy: Vec<UserFreeEvents> = Vec::with_capacity(service.users.len());

        for user in &service.users {
            let usecase = GetUserFreeBusyUseCase {
                calendar_ids: Some(user.calendar_ids.clone()),
                end_ts: booking_timespan.end_ts,
                start_ts: booking_timespan.start_ts,
                user_id: user.user_id.clone(),
            };

            let free_events = execute(usecase, &ctx).await;

            match free_events {
                Ok(free_events) => {
                    users_freebusy.push(UserFreeEvents {
                        free_events: free_events.free,
                        user_id: user.user_id.clone(),
                    });
                }
                Err(e) => {
                    println!("Error getting user freebusy: {:?}", e);
                }
            }
        }

        let booking_slots = get_service_bookingslots(
            users_freebusy,
            &BookingSlotsOptions {
                interval: self.interval,
                duration: self.duration,
                end_ts: booking_timespan.end_ts,
                start_ts: booking_timespan.start_ts,
            },
        );

        Ok(UseCaseRes { booking_slots })
    }
}
