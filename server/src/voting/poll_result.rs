use std::collections::HashMap;
use chrono::{DateTime, Utc};
use rand::{self, SeedableRng, rngs::StdRng, seq::SliceRandom};
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
    pub fn evaluate(poll: &'a Poll, ballots: Vec<&'a Ballot>, max_rounds: u32, rng_seed: &[u8; 32]) -> PollResult<'a> {
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
        ballots.shuffle(&mut StdRng::from_seed(*rng_seed));

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

                // drop ballot if exhausted
                if let Some(id) = selection {
                    // add this ballot to the list of votes for this option
                    let current_tally = tally.get_mut(id).unwrap();
                    current_tally.push(ballot);
                    println!("Tally for {id} is now {}", current_tally.len());
                    if current_tally.len() == threshold {
                        result.winners.push(id);
                    }
                }
                else {
                    println!("Dropping user {}", ballot.voter_id);
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

    static NAMES: [&str; 26] = [
        "Alice", "Bob", "Charlie", "David", "Eve", "Frank", "Grace", "Heidi", "Ivan", "Judy",
        "Kat", "Larry", "Mallory", "Nancy", "Oscar", "Peggy", "Quentin", "Randy", "Steve", "Trent",
        "Ursula", "Victor", "Wendy", "Xavier", "Yvonne", "Zelda",
    ];

    static RNG_SEED: [u8; 32] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15
    ];

    fn generate_poll(vote_prefs: Vec<Vec<u32>>) -> (Poll<'static>, Vec<PollOption>, Vec<User<'static>>, Vec<Ballot>) {
        assert!(vote_prefs.len() > 0);
        let option_count = vote_prefs[0].len();
        let voter_count = vote_prefs[0].iter().sum();

        let users: Vec<User> = (0..voter_count)
            .map(|i| User::new(i as u32, NAMES[i as usize]))
            .collect();
        let options: Vec<String> = (0..option_count)
            .map(|i| format!("Option {i}"))
            .collect();
        let (poll, options) = Poll::new(0, "Test Poll", options, 1, false, None, &users[0]);
        let mut ballots: Vec<Ballot> = vec![];

        // for each preference round (first, second, etc.)
        for (round_num, round) in vote_prefs.iter().enumerate() {

            // get the number of votes a particular option should get in this round
            for (option, option_vote_count) in round.iter().enumerate() {

                // cast the specified number of votes for this option
                for _ in 0..*option_vote_count {

                    // find a ballot with no vote for the current round, who hasn't already voted for this option
                    if let Some(ballot) = ballots.iter_mut().find(
                        |b| b.selection_ids.len() == round_num && !b.selection_ids.contains(&(option as u32))
                    ) {
                        ballot.selection_ids.push(option as u32);
                    }
                    // generate a new ballot if one doesn't yet exist
                    else if round_num == 0 {
                        let ballot = Ballot::new(&poll, &users[ballots.len()], vec![option as u32]);
                        ballots.push(ballot);
                    }
                    else {
                        panic!("Round {round_num} has more votes than the previous round");
                    }
                }
            }

            ballots.reverse();
        }

        (poll, options, users, ballots)
    }

    #[test]
    fn validate_poll_generator() {
        let (poll, options, users, ballots) = generate_poll(vec![
            vec![3, 2, 1],
            vec![2, 3],
        ]);

        assert_eq!(poll.option_ids, vec![0, 1, 2]);
        assert_eq!(options.len(), 3);
        assert_eq!(users.len(), 6);

        let selections = vec![
            vec![0, 1],
            vec![0, 1],
            vec![0, 1],
            vec![1],
            vec![1, 0],
            vec![2, 0],
        ];
        for (i, ballot) in ballots.iter().enumerate() {
            assert_eq!(ballot.selection_ids, selections[i], "Ballot {i} is not as expected");
        }
    }

    #[test]
    fn empty_poll_halts() {
        let (poll, _) = Poll::new(
            1,
            "Empty Poll",
            vec![],
            1,
            false,
            None,
            &User::new(0, "No user"),
        );
        let ballots = vec![];
        let result = PollResult::evaluate(&poll, ballots, 100, &RNG_SEED);
        assert_eq!(result.tally, vec![]);
        assert_eq!(result.winners, vec![] as Vec<&u32>);
        assert_eq!(result.eliminated, vec![] as Vec<&u32>);
    }

    #[test]
    fn simple_majority() {
        let (poll, _, _, ballots) = generate_poll(vec![
            vec![2, 1, 0, 0, 0],
        ]);

        let result = PollResult::evaluate(&poll, ballots.iter().collect(), 100, &RNG_SEED);
        assert_eq!(result.tally, &[(&0, 2), (&1, 1), (&2, 0), (&3, 0), (&4, 0)]);
        assert_eq!(result.winners, &[&0]);
        assert_eq!(result.eliminated, vec![] as Vec<&u32>);
    }

    #[test]
    fn simple_three_rounds() {
        let (poll, _, _, ballots) = generate_poll(vec![
            vec![4, 3, 2, 1],
            vec![4, 4, 2],
        ]);

        dbg!(&ballots);

        let result = PollResult::evaluate(&poll, ballots.iter().collect(), 100, &RNG_SEED);
        assert_eq!(result.winners, &[&0], "Winners correct");
        assert_eq!(result.eliminated, &[&3, &2], "Eliminated correct");
    }

    // #[test]
    // fn tied_elim() {
    //     let (poll, _, _, ballots) = generate_poll(vec![
    //         vec![1, 1, 1, 0, 0],
    //         vec![1],
    //     ]);

    //     dbg!(&ballots);

    //     let result = PollResult::evaluate(&poll, ballots.iter().collect(), 100);
    //     assert_eq!(result.tally, &[(&0, 2), (&1, 1), (&2, 0), (&3, 0), (&4, 0)]);
    //     assert_eq!(result.winners, &[&0]);
    //     assert_eq!(result.eliminated, &[&2]);
    // }
}
