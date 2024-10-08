use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};

use chrono::{DateTime, Utc};
use rand::{self, SeedableRng, rngs::StdRng, prelude::SliceRandom};
use serde::Serialize;

use super::ballot::Ballot;
use super::id::{Id, WeakId};
use super::poll::Poll;

/// A displayable version of HashMap<&u32, Vec<&Ballot>>
struct Tally<'a>(&'a HashMap<&'a WeakId, Vec<&'a Ballot>>);
impl<'a> Display for Tally<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut sorted_tally: Vec<TallyItem> = self.0.iter()
            .map(|(id, votes)| {
                TallyItem { option_id: **id, vote_count: votes.len() as u32 }
            })
            .collect();
        sorted_tally.sort();

        write!(f, "Tally [")?;
        for item in sorted_tally {
            write!(f, "{item}, ")?;
        }
        write!(f, "]")
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct TallyItem {
    option_id: WeakId,
    vote_count: u32,
}

impl TallyItem {
    pub fn new(id: u32, count: u32) -> Self {
        Self {
            option_id: WeakId(id),
            vote_count: count,
        }
    }
}

impl Display for TallyItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({}: {})", self.option_id, self.vote_count)
    }
}

impl PartialOrd for TallyItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TallyItem {
    /// Sorts by vote count descending, then id ascending
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.vote_count.cmp(&other.vote_count).reverse().then(self.option_id.cmp(&other.option_id))
    }
}

/// A displayable version of Vec<&Ballot>
struct BallotList<'a>(pub &'a [Ballot]);
impl<'a> std::fmt::Display for BallotList<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "BallotList [")?;
        for ballot in self.0.iter() {
            write!(f, "{}, ", ballot)?;
        }
        write!(f, "]")
    }
}

#[derive(Serialize)]
pub struct PollResult {
    pub poll_id: Id,
    pub poll: Option<Poll>,
    pub evaluated_at: DateTime<Utc>,

    pub threshold: usize,
    pub tally: Vec<TallyItem>,
    pub winners: Vec<WeakId>,
    pub eliminated: Vec<WeakId>,
}

impl PollResult {
    pub fn evaluate(poll: &Poll, ballots: &[Ballot], max_rounds: u32, rng_seed: &[u8; 32]) -> PollResult {
        println!("{}", BallotList(&ballots));

        let mut result = PollResult {
            poll_id: poll.id.clone(),
            poll: Some(poll.clone()),
            evaluated_at: Utc::now(),
            threshold: ballots.len() / (poll.winner_count as usize + 1) + 1,
            tally: vec![],
            winners: vec![],
            eliminated: vec![],
        };

        // abort tallying if there are not enough votes to determine a winner
        if result.threshold > ballots.len() {
            return result;
        }

        // clone the list of ballots so we can shuffle and throw out invalid/settled/exhausted ballots
        let mut ballots = Vec::from_iter(ballots.iter());
        ballots.shuffle(&mut StdRng::from_seed(*rng_seed));

        let vecs = poll.option_ids.iter().map(|_| vec![]);
        let mut tally = poll.option_ids.iter().zip(vecs).collect::<HashMap<&WeakId, Vec<&Ballot>>>();

        // calculate the overall popularity of each option (1 first pref ~== 2 second prefs ~== 4 third prefs)
        let mut popularity: HashMap<&WeakId, f64> = HashMap::new();
        for ballot in ballots.iter() {
            for (pref, option) in ballot.ranked_preferences.iter().enumerate() {
                popularity.insert(
                    option,
                    popularity.get(option).unwrap_or(&0f64) + (1f64 / (pref as f64 + 1f64)));
            }
        }

        for round in 1..=max_rounds {
            // count the votes for each option
            while let Some(ballot) = ballots.pop() {
                // find the vote from this ballot
                let selection = ballot.ranked_preferences.iter()
                    .find(|id| !result.eliminated.contains(id) && !result.winners.contains(id));
                println!("User {:?} votes for {selection:?}", ballot.voter);

                // drop ballot if exhausted
                if let Some(id) = selection {
                    // add this ballot to the list of votes for this option
                    let current_tally = tally.get_mut(id).unwrap();
                    current_tally.push(ballot);
                    if current_tally.len() == result.threshold {
                        result.winners.push(id.clone());
                    }
                }
            }

            println!("{}", Tally(&tally));

            if result.winners.len() > poll.winner_count as usize {
                panic!("How did we get too many winners?");
            }
            else if result.winners.len() == poll.winner_count as usize {
                println!("Winners: {:?}", result.winners);
                break;
            }
            // find the option with the fewest votes, breaking ties by popularity
            else if let Some(min_votes) = tally.iter().map(|(_, votes)| votes.len()).min() {
                let loser = tally.iter()
                    .filter(|(_, votes)| votes.len() == min_votes)
                    .min_by(|(a, _), (b, _)| {
                        let a_pop = *popularity.get(*a).unwrap_or(&0f64);
                        let b_pop = *popularity.get(*b).unwrap_or(&0f64);
                        a_pop.partial_cmp(&b_pop).unwrap() // panics on NaN
                    }).unwrap().0;
                println!("No winner after round {round}, eliminating {loser}");
                result.eliminated.push((*loser).clone());
                ballots = tally.remove(*loser).unwrap();
            }
            else {
                println!("No ballots remaining, inconclusive");
                break;
            }
        }

        // fill back in eliminated options with zero votes
        result.tally = poll.option_ids.iter()
            .map(|id| {
                TallyItem {
                    option_id: *id,
                    vote_count: match tally.get(id) {
                        Some(votes) => votes.len() as u32,
                        None => 0,
                    }
                }
            })
            .collect();
        // sort by number of votes descending, then by id ascending
        result.tally.sort();

        result
    }

}

#[cfg(test)]
mod tests {
    use super::super::*;

    static RNG_SEED: [u8; 32] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15
    ];

    /// Generate a poll, options, voters, and ballots from a list of vote preferences
    fn generate_poll(winner_count: u8, mut vote_prefs: Vec<Vec<u32>>) -> (Poll, Vec<Ballot>) {
        let voter_count = vote_prefs.len();
        let mut option_count = 0;
        for ballot in &vote_prefs {
            for vote in ballot {
                option_count = option_count.max(*vote);
            }
        }

        let mut users: Vec<User> = (0..voter_count)
            .map(|i| User::new(Id::new(), format!("Voter {i}")))
            .collect();
        let options: Vec<String> = (0..=option_count)
            .map(|i| format!("Option {i}"))
            .collect();
        let poll = Poll::from(CreatePollSettings {
            id: None,
            title: String::from("Test Poll"),
            options,
            winner_count,
            write_ins_allowed: false,
            close_after_time: None,
            close_after_votes: None,
        });

        let mut ballots = vec![];

        while let Some(prefs) = vote_prefs.pop() {
            let ballot = Ballot::new(
                users.pop().unwrap(),
                CreateBallot {
                    ranked_preferences: prefs.iter().map(|i| WeakId(*i)).collect(),
                    ..CreateBallot::default()
                },
            );
            ballots.push(ballot);
        }
        ballots.reverse();

        (poll, ballots)
    }

    #[test]
    fn validate_poll_generator() {
        let (poll, ballots) = generate_poll(1, vec![
            vec![3, 2, 1],
            vec![2, 3],
        ]);

        assert_eq!(poll.option_ids, vec![0, 1, 2, 3], "Check option ids");
        assert_eq!(poll.options.unwrap().len(), 4, "Check option count");
        assert_eq!(ballots.len(), 2, "Check ballot count");
        assert_eq!(ballots[0].ranked_preferences, vec![3, 2, 1], "Check ballot 1");
        assert_eq!(ballots[1].ranked_preferences, vec![2, 3], "Check ballot 2");
    }

    #[test]
    fn empty_poll_halts() {
        let poll = Poll::from(CreatePollSettings {
            id: None,
            title: String::from("Empty Poll"),
            options: vec![],
            winner_count: 1,
            write_ins_allowed: false,
            close_after_time: None,
            close_after_votes: None,
        });
        let ballots = vec![];
        let result = PollResult::evaluate(&poll, &ballots, 1, &RNG_SEED);
        assert_eq!(result.winners, vec![] as Vec<WeakId>, "Check winners");
        assert_eq!(result.eliminated, vec![] as Vec<WeakId>, "Check eliminated");
        assert_eq!(result.tally, vec![], "Check tally");
    }

    #[test]
    fn simple_majority() {
        let (poll, ballots) = generate_poll(1, vec![
            vec![2],
            vec![1],
            vec![1],
        ]);

        let result = PollResult::evaluate(&poll, ballots.as_ref(), 1, &RNG_SEED);
        assert_eq!(result.winners, &[1], "Check winners");
        assert_eq!(result.eliminated, vec![] as Vec<WeakId>, "Check eliminated");
        assert_eq!(result.tally, &[
            TallyItem::new(1, 2),
            TallyItem::new(2, 1),
            TallyItem::new(0, 0)],
            "Check tally");
    }

    #[test]
    fn simple_two_rounds() {
        let (poll, ballots) = generate_poll(1, vec![
            // 5 votes, 1 seat = 3 votes to win
            vec![0],
            vec![0],
            vec![1],
            vec![1],
            vec![2, 0],
        ]);

        let result = PollResult::evaluate(&poll, ballots.as_ref(), 2, &RNG_SEED);
        assert_eq!(result.winners, &[0], "Check winners");
        assert_eq!(result.eliminated, &[2], "Check eliminated");
        assert_eq!(result.tally, &[
            TallyItem::new(0, 3),
            TallyItem::new(1, 2),
            TallyItem::new(2, 0)],
            "Check tally");
    }

    #[test]
    fn tied_elim() {
        let (poll, ballots) = generate_poll(1, vec![
            vec![0],
            vec![0, 1],
            vec![1, 0],
            vec![2, 0],
        ]);

        let result = PollResult::evaluate(&poll, ballots.as_ref(), 2, &RNG_SEED);
        assert_eq!(result.winners, &[0], "Check winners");
        assert_eq!(result.eliminated, &[2], "Check eliminated");
        assert_eq!(result.tally, &[
            TallyItem::new(0, 3),
            TallyItem::new(1, 1),
            TallyItem::new(2, 0)],
            "Check tally");
    }

    #[test]
    fn two_winners_simple() {
        let (poll, ballots) = generate_poll(2, vec![
            vec![0],
            vec![1],
        ]);

        let result = PollResult::evaluate(&poll, ballots.as_ref(), 1, &RNG_SEED);
        assert_eq!(result.winners, &[0, 1], "Check winners");
        assert_eq!(result.eliminated, vec![] as Vec<WeakId>, "Check eliminated");
        assert_eq!(result.tally, &[
            TallyItem::new(0, 1),
            TallyItem::new(1, 1)],
            "Check tally");
    }

    #[test]
    fn two_winners_two_rounds() {
        let (poll, ballots) = generate_poll(2, vec![
            // 6 votes, 2 seats = 3 votes to win
            vec![0],
            vec![0],
            vec![1],
            vec![1],
            vec![2, 0],
            vec![2, 1],
        ]);

        let result = PollResult::evaluate(&poll, ballots.as_ref(), 2, &RNG_SEED);
        assert_eq!(result.winners, &[1, 0], "Check winners");
        assert_eq!(result.eliminated, &[2], "Check eliminated");
        assert_eq!(result.tally, &[
            TallyItem::new(0, 3),
            TallyItem::new(1, 3),
            TallyItem::new(2, 0)],
            "Check tally");
    }
}
