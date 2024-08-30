use std::collections::HashMap;
use chrono::{DateTime, Utc};
use rand::{self, seq::SliceRandom};
use serde::Serialize;
use uuid::Uuid;
use crate::poll::{Poll, Ballot};

#[derive(Serialize, Debug)]
pub struct PollResult<'a> {
    pub poll_id: &'a Uuid,
    pub evaluated_at: DateTime<Utc>,

    pub tally: Vec<(&'a Uuid, u32)>,
    pub winners: Vec<&'a Uuid>,
    pub eliminated: Vec<&'a Uuid>,
}

impl<'a> PollResult<'a> {
    pub fn evaluate(poll: &'a Poll, ballots: &'a [&'a Ballot], max_rounds: u32) -> PollResult<'a> {
        let mut result = PollResult {
            poll_id: &poll.id,
            evaluated_at: Utc::now(),
            tally: vec![],
            winners: vec![],
            eliminated: vec![],
        };

        // abort tallying if there are not enough votes to determine a winner
        let threshold = ballots.len() / (poll.winner_count as usize + 1) + 1;
        if threshold > ballots.len() {
            return result;
        }

        // clone the list of ballots so we can shuffle and throw out invalid/settled/exhausted ballots
        let mut ballots = ballots.to_owned();
        ballots.shuffle(&mut rand::thread_rng());

        let vecs = poll.option_ids.iter().map(|_| vec![]);
        let mut tally = poll.option_ids.iter().zip(vecs).collect::<HashMap<&Uuid, Vec<&Ballot>>>();

        for _ in 0..max_rounds {
            // count the votes for each option
            loop {
                let ballot = match ballots.pop() {
                    Some(b) => b,
                    None => break,
                };

                // reject if the ballot is not for the poll being evaluated
                if ballot.poll_id != poll.id {
                    continue;
                }

                // find the vote from this ballot
                let selection = ballot.selection_ids.iter()
                    .find(|id| !result.eliminated.contains(id) && !result.winners.contains(id));

                match selection {
                    // drop this ballot if it has no remaining votes to cast
                    None => continue,
                    Some(id) => {
                        // add this ballot to the list of votes for this option
                        let current_tally = tally.get_mut(id).unwrap();
                        current_tally.push(ballot);
                        if current_tally.len() == threshold {
                            result.winners.push(id);
                        }
                    },
                }
            }

            if result.winners.len() > poll.winner_count as usize {
                panic!("How did we get too many winners?");
            }
            else if result.winners.len() == poll.winner_count as usize {
                break;
            }
            else {
                let loser = tally.iter().min_by_key(|(_, votes)| votes.len()).unwrap().0;
                result.eliminated.push(*loser);
                ballots = tally.remove(*loser).unwrap();
            }
        }

        result.tally = tally.iter().map(|(id, votes)| (*id, votes.len() as u32)).collect();

        result
    }

}

#[cfg(test)]
mod tests {
    use crate::poll::{Poll, Ballot, User};
    use super::*;

    static OPTIONS: [&str; 5] = [
        "Option 1", "Option 2", "Option 3", "Option 4", "Option 5",
    ];

    static USERS: [User; 5] = [
        User::new("1", "Alice"),
        User::new("2", "Bob"),
        User::new("3", "Charlie"),
        User::new("4", "David"),
        User::new("5", "Eve"),
    ];

    #[test]
    fn empty_poll_halts() {
        let (poll, options) = Poll::new(
            "Empty Poll",
            &OPTIONS,
            1,
            false,
            None,
            USERS[0],
        );
        let ballots = vec![];
        let result = PollResult::evaluate(&poll, &ballots, 100);
        assert_eq!(result.tally.len(), 0);
    }
}
