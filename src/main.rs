extern crate chrono;
extern crate reqwest;
#[macro_use]
extern crate serde_json;

use std::collections::HashMap;
use std::env;
use std::io::Read;

use chrono::{Duration, UTC};

use serde_json::Value;

const MEETUPS_API_HOST: &'static str = "https://api.meetup.com";
const GROUPS_ENDPOINT: &'static str = "/2/groups";
const CONVERSATION_ENDPOINT: &'static str = "/self/conversation";

/// Search through the first 500 results in meetup.com for Rust
/// and filter it preserving those compliant with the following criteria:
///
/// * Older than 12 weeks and have had an event in the last 12 weeks
/// * Rating above 4.0
/// * Have more than 12 members

fn main() {
    let api_key = env::var("MEETUP_API_KEY")
        .expect("MEETUP_API_KEY env var not found");

    let groups_url = format!(
        "{}{}?topic=rust&fields=last_event&limit=500&key={}",
        MEETUPS_API_HOST,
        GROUPS_ENDPOINT,
        api_key
    );

    let conversations_url = format!(
        "{}{}?key={}",
        MEETUPS_API_HOST,
        CONVERSATION_ENDPOINT,
        api_key
    );

    let res = reqwest::get(&groups_url);

    // Whitelist the meetups we do want to contact
    let mut confirmed_meetups = HashMap::<u64, bool>::new();
    // confirmed_meetups.insert(10495542, true); // Bay Area
    // confirmed_meetups.insert(13280632, true); // Rust London
    // confirmed_meetups.insert(14549592, true); // Rust Paris
    // confirmed_meetups.insert(17841102, true); // Columbus Rust Society
    // confirmed_meetups.insert(18206588, true); // Boston Rust
    // confirmed_meetups.insert(18215915, true); // Rust Amsterdam
    // confirmed_meetups.insert(18350214, true); // Rust NYC
    // confirmed_meetups.insert(18371011, true); // San Diego Rust
    // confirmed_meetups.insert(18382460, true); // Rust Sydney
    // confirmed_meetups.insert(18476143, true); // Rust Melbourne
    // confirmed_meetups.insert(18553572, true); // PDXRust
    // confirmed_meetups.insert(18572150, true); // Rust Boulder/Denver
    // confirmed_meetups.insert(18603407, true); // Rust Los Angeles
    // confirmed_meetups.insert(18637995, true); // Rust Cologne
    // confirmed_meetups.insert(18678574, true); // Singapore Rust Meetup
    // confirmed_meetups.insert(18742098, true); // Rust Meetup Hamburg
    // confirmed_meetups.insert(18782559, true); // Rust Dublin
    // confirmed_meetups.insert(18810032, true); // Rust Detroit
    // confirmed_meetups.insert(18934389, true); // Seattle Rust Meetup
    // confirmed_meetups.insert(18998571, true); // Rust Rhein Main
    // confirmed_meetups.insert(19172079, true); // Rust DC
    // confirmed_meetups.insert(19884717, true); // Rust Brisbane
    // confirmed_meetups.insert(19935700, true); // Rust Stockholm
    // confirmed_meetups.insert(19961179, true); // Rust Prague
    confirmed_meetups.insert(20018153, true); // Rust MX
    // confirmed_meetups.insert(20177234, true); // Rust Atlanta
    // confirmed_meetups.insert(20367210, true); // Rust Leipzig
    // confirmed_meetups.insert(20471787, true); // South Florida Rust Meetup
    // confirmed_meetups.insert(21077451, true); // Rust Utrecht
    // confirmed_meetups.insert(21656077, true); // Rust Rotterdam
    // confirmed_meetups.insert(21699516, true); // Rust Roma

    let meetups_to_contact: Vec<Value> = match res {
        Ok(mut r) => {
            let mut resp = String::new();
            r.read_to_string(&mut resp).unwrap();

            let json: Value = serde_json::from_str(&resp)
                .expect("failed to parse JSON response");

            let created_limit = UTC::now() - Duration::weeks(12);
            let created_limit_ts: u64 = (created_limit.timestamp() * 1000) as u64
                + (created_limit.timestamp_subsec_millis() as u64);

            let last_event_limit = UTC::now() - Duration::weeks(12);
            let last_event_limit_ts: u64 = (last_event_limit.timestamp() * 1000) as u64
                + (last_event_limit.timestamp_subsec_millis() as u64);

            let results = json.get("results")
                .expect("no results in the response")
                .as_array()
                .expect("failed to parse results as array");

            results.iter()
                .filter_map(|val| {
                    let id = val.get("id").unwrap().as_u64().unwrap();
                    let confirmed = confirmed_meetups.get(&id).is_some();
                    if !confirmed {
                        return None;
                    }

                    let rating = val.get("rating").unwrap().as_f64().unwrap();
                    let created = val.get("created").unwrap().as_u64().unwrap();
                    let last_event_ts = match val.get("last_event") {
                        Some(&Value::Object(ref le)) => le.get("time").unwrap().as_u64().unwrap(),
                        Some(_) => 0u64,
                        None => 0u64
                    };
                    let member_count = val.get("members").unwrap().as_u64().unwrap();

                    let test = rating > 4.0 &&
                        created < created_limit_ts &&
                        last_event_ts > last_event_limit_ts &&
                        member_count > 12;

                    match test {
                        true => {
                            let organizer = val.get("organizer").unwrap();
                            let name = organizer.get("name").unwrap().to_owned();
                            let member_id = organizer.get("member_id").unwrap()
                                .as_u64().unwrap();
                            let member_id_str = format!("{}", member_id.to_owned());

                            let link = val.get("link")
                                .unwrap()
                                .as_str()
                                .unwrap()
                                .to_owned();

                            let mut conversation_args = HashMap::new();
                            conversation_args.insert("title", "");
                            conversation_args.insert("text", "Automated test from Rust");
                            conversation_args.insert("member", &member_id_str);
                            conversation_args.insert("conversation_kind", "one_one");
                            conversation_args.insert("photo_host", "secure");

                            println!("Contacting {} ({}) from {}...", name, member_id, link);
                            let conversation = reqwest::Client::new()
                                .unwrap()
                                .post(&conversations_url)
                                .send();
                            match conversation {
                                Ok(mut r0) => println!("Message successfuly sent: {:?}...", r0.json::<serde_json::Value>().unwrap()),
                                Err(e) => println!("Failure sending message: {:?}...", e)
                            }

                            Some(json!({
                                "id": id,
                                "url": link,
                                "organizer": json!({
                                    "id": member_id,
                                    "url": format!("https://www.meetup.com/members/{}", member_id),
                                    "name": name
                                })
                            }))

                        },
                        false => {
                            None
                        }
                    }
                })
                .collect()
        },
        Err(e) => {
            println!("err: {:?}", e);
            vec![]
        }
    };

    println!("{}", serde_json::to_string_pretty(&json!({
        "results": meetups_to_contact
    })).unwrap());
}
