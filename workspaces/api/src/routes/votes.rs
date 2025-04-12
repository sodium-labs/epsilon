use crate::environment::{ApiState, Environment};
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    Json,
};
use database::{
    models::NewVote,
    schema::{pages, votes},
    types::VoteType,
};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl};
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use utils::sql::get_sql_timestamp;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn create_votes_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new().routes(routes!(post_vote_handler))
}

#[derive(Deserialize)]
struct VoteBody {
    page_url: String,
    fingerprint: String,
    vote_type: i32,
}

#[utoipa::path(
    post,
    path = "",
    description = "Vote for a page",
    responses(
        (status = OK),
        (status = BAD_REQUEST),
        (status = UNAUTHORIZED)
    )
)]
#[axum::debug_handler]
async fn post_vote_handler(
    State(state): State<Arc<Environment>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<VoteBody>,
) -> StatusCode {
    if payload.page_url.len() > 2048 {
        return StatusCode::BAD_REQUEST;
    }

    let db_conn = &mut state.db_pool.get().unwrap();

    if let Some(page_id) = pages::table
        .select(pages::id)
        .filter(pages::url.eq(&payload.page_url))
        .get_result::<i32>(db_conn)
        .optional()
        .unwrap()
    {
        let ip = addr.ip();
        let ip_str = ip.to_string();

        // 0 = remove the vote, so we try to delete it if present
        if payload.vote_type == 0 {
            diesel::delete(
                votes::table
                    .filter(votes::page_id.eq(page_id))
                    .filter(votes::fingerprint.eq(&payload.fingerprint)),
            )
            .execute(db_conn)
            .unwrap();

            return StatusCode::OK;
        }

        let new_vote_type = VoteType::try_from(payload.vote_type);
        if new_vote_type.is_err() {
            return StatusCode::BAD_REQUEST;
        }

        let existing_vote: Option<i32> = votes::table
            .filter(votes::page_id.eq(page_id))
            .filter(votes::fingerprint.eq(&payload.fingerprint))
            .select(votes::id)
            .first(db_conn)
            .optional()
            .unwrap();

        if existing_vote.is_none() {
            // Check the number of vote by this IP
            let ip_vote_count = votes::table
                .filter(votes::page_id.eq(page_id))
                .filter(votes::ip.eq(&ip_str))
                .count()
                .get_result::<i64>(db_conn)
                .unwrap();

            // Limited to 10 votes by IP
            if ip_vote_count >= 10 {
                return StatusCode::UNAUTHORIZED;
            }
        }

        let now_timestamp = get_sql_timestamp();

        // Insert or update the vote
        diesel::insert_into(votes::table)
            .values(NewVote {
                ip: ip_str,
                page_id,
                fingerprint: payload.fingerprint,
                vote_type: new_vote_type.clone().unwrap(),
                updated_at: now_timestamp,
                created_at: now_timestamp,
            })
            .on_conflict((votes::page_id, votes::fingerprint))
            .do_update()
            .set((
                votes::vote_type.eq(new_vote_type.unwrap()),
                votes::updated_at.eq(now_timestamp),
            ))
            .execute(db_conn)
            .unwrap();

        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    }
}
