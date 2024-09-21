use diesel::prelude::*;
use uuid::Uuid;
use warp::reply::{self, Reply, Response};
use warp::http::StatusCode;

use crate::voting;
use super::db::{establish_connection, schema, models};
use super::poll_api::get_internal as get_poll;

pub fn get_result(poll_id: Uuid) -> Response {
    let conn = &mut establish_connection();

    // fetch poll
    let poll = match get_poll(conn, &poll_id) {
        Err(err) => { return err.into_response(); },
        Ok(p) => p,
    };

    let result = schema::ballots::table
        .inner_join(schema::votes::table)
        .filter(schema::ballots::poll_id.eq(poll_id))
        .order((schema::ballots::id, schema::votes::preference))
        .select(models::Vote::as_select())
        .load(conn);
    let votes: Vec<models::Vote> = match result {
        Err(err) => {
            return reply::with_status(
                format!("Failed to fetch ballots for poll {poll_id}: {err}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ).into_response();
        },
        Ok(v) => v,
    };
    println!("Found {} votes", votes.len());

    if votes.len() < 3 {
        return reply::with_status("Not yet enough votes to tally", StatusCode::NO_CONTENT).into_response();
    }

    let mut ballots = vec![];
    let mut ballot = (
        votes[0].ballot_id,
        voting::Ballot { ranked_preferences: vec![], ..Default::default() },
    );
    for vote in votes {
        println!("Tally me banana!");
        if vote.ballot_id != ballot.0 {
            ballots.push(ballot.1);
            ballot = (
                vote.ballot_id,
                voting::Ballot { ranked_preferences: vec![], ..Default::default() },
            );
        }

        ballot.1.ranked_preferences.push(voting::WeakId(vote.option as u32));
    }
    ballots.push(ballot.1);
    println!("Collated to {} ballots", ballots.len());

    let result = voting::PollResult::evaluate(
        &poll,
        ballots.as_ref(),
        poll.option_ids.len() as u32 - poll.winner_count as u32,
        &poll.rng_seed);

    reply::json(&result).into_response()
}
