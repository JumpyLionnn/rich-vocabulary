// https://github.com/dulldesk/words-api/tree/master - amount, first letter, kind (noun or adj) // bad because it sends duplicates
// https://random-word-api.vercel.app/ - amount, length, first letter
// https://random-word.ryanrk.com/ - amount, length(minmax) // bad because the words are weird

use crate::DictionaryError;

const RANDOM_WORD_API_URL: &'static str = "https://random-word-api.vercel.app/api";

pub(crate) async fn get_random_words(
    client: &reqwest::Client,
    max: usize,
    length: Option<usize>,
) -> Result<Vec<String>, DictionaryError> {
    let mut req = client
        .get(RANDOM_WORD_API_URL)
        .query(&[("words", max)]);
    if let Some(length) = length {
        req = req.query(&[("length", length)]);
    }
    let res: reqwest::Response = req.send().await.map_err(DictionaryError::Fetch)?;
    res.json::<Vec<String>>()
        .await
        .map_err(DictionaryError::Deserialize)
}
