use std::collections::HashMap;
use chrono::{DateTime, Utc};
use rand::{self, seq::SliceRandom};
use serde::Serialize;
use uuid::Uuid;
use super::poll::{Poll, Ballot};

#[derive(Serialize, Debug)]
pub struct PollResult<'a> {
    pub poll_id: &'a u32,
    pub evaluated_at: DateTime<Utc>,

    pub tally: Vec<(&'a u32, u32)>,
    pub winners: Vec<&'a u32>,
    pub eliminated: Vec<&'a u32>,
}

impl<'a> PollResult<'a> {
    pub fn evaluate(poll: &'a Poll, ballots: Vec<&'a Ballot>, max_rounds: u32) -> PollResult<'a> {
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
        let mut ballots = ballots;
        ballots.shuffle(&mut rand::thread_rng());

        let vecs = poll.option_ids.iter().map(|_| vec![]);
        let mut tally = poll.option_ids.iter().zip(vecs).collect::<HashMap<&u32, Vec<&Ballot>>>();

        // calculate the overall popularity of each option
        let mut popularity: HashMap<&u32, f64> = HashMap::new();
        for ballot in ballots.iter() {
            for (pref, option) in ballot.selection_ids.iter().enumerate() {
                popularity.insert(
                    option,
                    popularity.get(option).unwrap_or(&0f64) + (1f64 / (pref as f64 + 1f64)));
            }
        }

        for round in 1..=max_rounds {
            // count the votes for each option
            while let Some(ballot) = ballots.pop() {
                // reject if the ballot is not for the poll being evaluated
                if ballot.poll_id != poll.id {
                    continue;
                }

                // find the vote from this ballot
                let selection = ballot.selection_ids.iter()
                    .find(|id| !result.eliminated.contains(id) && !result.winners.contains(id));
                println!("User {} votes for {selection:?}", ballot.voter_id);

                match selection {
                    // drop this ballot if it has no remaining votes to cast
                    None => continue,
                    Some(id) => {
                        // add this ballot to the list of votes for this option
                        let current_tally = tally.get_mut(id).unwrap();
                        current_tally.push(ballot);
                        println!("Tally for {id} is now {}", current_tally.len());
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
                println!("Winners: {:?}", result.winners);
                break;
            }
            else {
                // find the option with the fewest votes, breaking ties by popularity
                let min_votes = tally.iter().map(|(_, votes)| votes.len()).min().unwrap();
                let loser = tally.iter()
                    .filter(|(_, votes)| votes.len() == min_votes)
                    .min_by(|(a, _), (b, _)| {
                        let a_pop = *popularity.get(*a).unwrap_or(&0f64);
                        let b_pop = *popularity.get(*b).unwrap_or(&0f64);
                        a_pop.partial_cmp(&b_pop).unwrap() // panics on NaN
                    }).unwrap().0;
                println!("No winner after round {round}, eliminating {loser}");
                result.eliminated.push(*loser);
                ballots = tally.remove(*loser).unwrap();
            }
        }

        // fill back in eliminated options with zero votes
        result.tally = poll.option_ids.iter()
            .map(|id| {
                (id, match tally.get(id) {
                    Some(votes) => votes.len() as u32,
                    None => 0,
                })
            })
            .collect();
        // sort by number of votes descending, then by id ascending
        result.tally.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));

        result
    }

}

#[cfg(test)]
mod tests {
    use super::super::poll::*;
    use super::*;

    static OPTIONS: [&str; 5] = [
        "Option 1", "Option 2", "Option 3", "Option 4", "Option 5",
    ];

    static USERS: [User; 5] = [
        User::new(1, "1", "Alice"),
        User::new(2, "2", "Bob"),
        User::new(3, "3", "Charlie"),
        User::new(4, "4", "David"),
        User::new(5, "5", "Eve"),
    ];

    #[test]
    fn empty_poll_halts() {
        let (poll, _) = Poll::new(
            1,
            "Empty Poll",
            &OPTIONS,
            1,
            false,
            None,
            &USERS[0],
        );
        let ballots = vec![];
        let result = PollResult::evaluate(&poll, ballots, 100);
        assert_eq!(result.tally.len(), 0);
    }

    #[test]
    fn simple_majority() {
        let (poll, _) = Poll::new(
            1,
            "Majority",
            &OPTIONS,
            1,
            false,
            None,
            &USERS[0],
        );
        let ballots = vec![
            Ballot::new(&poll, &USERS[0], vec![0]),
            Ballot::new(&poll, &USERS[1], vec![0]),
            Ballot::new(&poll, &USERS[2], vec![1]),
        ];

        let result = PollResult::evaluate(&poll, ballots.iter().collect(), 100);
        assert_eq!(result.tally, &[(&0, 2), (&1, 1), (&2, 0), (&3, 0), (&4, 0)]);
        assert_eq!(result.winners, &[&0]);
    }

    #[test]
    fn two_rounds() {
        let (poll, _) = Poll::new(
            1,
            "Two rounds",
            &OPTIONS,
            1,
            false,
            None,
            &USERS[0],
        );
        let ballots = vec![
            Ballot::new(&poll, &USERS[0], vec![0]),
            Ballot::new(&poll, &USERS[1], vec![0]),
            Ballot::new(&poll, &USERS[2], vec![1]),
            Ballot::new(&poll, &USERS[3], vec![1]),
            Ballot::new(&poll, &USERS[4], vec![2, 0]),
        ];

        let result = PollResult::evaluate(&poll, ballots.iter().collect(), 100);
        assert_eq!(result.tally, &[(&0, 3), (&1, 2), (&2, 0), (&3, 0), (&4, 0)]);
        assert_eq!(result.winners, &[&0]);
    }

    #[test]
    fn tied_elim() {
        let (poll, _) = Poll::new(
            1,
            "Two rounds with tied losers",
            &OPTIONS,
            1,
            false,
            None,
            &USERS[0],
        );
        let ballots = vec![
            Ballot::new(&poll, &USERS[0], vec![0]),
            Ballot::new(&poll, &USERS[1], vec![1]),
            Ballot::new(&poll, &USERS[2], vec![2, 0]),
        ];

        let result = PollResult::evaluate(&poll, ballots.iter().collect(), 100);
        assert_eq!(result.tally, &[(&0, 2), (&1, 1), (&2, 0), (&3, 0), (&4, 0)]);
        assert_eq!(result.winners, &[&0]);
    }
}
