use serde::Deserialize;

const URL: &'static str = "https://api.dictionaryapi.dev/api/v2/entries/en/";

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ApiResponse {
    Found(Vec<ApiWord>),
    Error(ApiError),
}

#[derive(Debug, Deserialize)]
struct ApiError {
    pub title: String,
    pub message: String,
    pub resolution: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiWord {
    pub word: String,
    pub phonetic: Option<String>,
    pub phonetics: Vec<ApiPhonetic>,
    pub origin: Option<String>,
    pub meanings: Vec<ApiWordMeaning>,
}

#[derive(Debug, Deserialize)]
pub struct ApiPhonetic {
    pub text: Option<String>,
    pub audio: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApiWordMeaning {
    #[serde(rename = "partOfSpeech")]
    pub part_of_speech: String,
    pub definitions: Vec<ApiWordDefinition>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApiWordDefinition {
    pub definition: String,
    pub example: Option<String>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
}

#[derive(Debug)]
pub struct Word {
    pub word: String,
    pub phonetic: Option<String>,
    pub phonetics: Vec<Phonetic>,
    pub origin: Option<String>,
    pub meanings: Vec<WordMeaning>,
}

impl TryFrom<ApiWord> for Word {
    type Error = UnknownPartOfSpeech;

    fn try_from(word: ApiWord) -> Result<Self, Self::Error> {
        type Partitioned = Vec<Result<WordMeaning, UnknownPartOfSpeech>>;
        let (meanings, mut errors): (Partitioned, Partitioned) = word
            .meanings
            .into_iter()
            .map(WordMeaning::try_from)
            .partition(Result::is_ok);
        if let Some(err) = errors.pop() {
            Err(err.unwrap_err())
        } else {
            Ok(Self {
                word: word.word,
                phonetic: word.phonetic,
                phonetics: word.phonetics.into_iter().map(Phonetic::from).collect(),
                origin: word.origin,
                meanings: meanings.into_iter().map(Result::unwrap).collect(),
            })
        }
    }
}

#[derive(Debug)]
pub struct Phonetic {
    pub text: Option<String>,
    pub audio: Option<String>,
}

impl From<ApiPhonetic> for Phonetic {
    fn from(phonetic: ApiPhonetic) -> Self {
        Self {
            text: phonetic.text,
            audio: phonetic
                .audio
                .and_then(|src| if src.is_empty() { None } else { Some(src) }),
        }
    }
}

#[derive(Debug)]
pub struct WordMeaning {
    pub part_of_speech: PartOfSpeech,
    pub definitions: Vec<WordDefinition>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
}

impl TryFrom<ApiWordMeaning> for WordMeaning {
    type Error = UnknownPartOfSpeech;

    fn try_from(meaning: ApiWordMeaning) -> Result<Self, Self::Error> {
        Ok(Self {
            part_of_speech: meaning.part_of_speech.try_into()?,
            definitions: meaning
                .definitions
                .into_iter()
                .map(WordDefinition::from)
                .collect(),
            synonyms: meaning.synonyms,
            antonyms: meaning.antonyms,
        })
    }
}

#[derive(Debug)]
pub enum PartOfSpeech {
    Noun,
    Pronoun,
    Verb,
    Adjective,
    Adverb,
    Preposition,
    Conjunction,
    Interjection,
}

#[derive(Debug)]
pub struct UnknownPartOfSpeech {
    pub kind: String,
}

impl TryFrom<String> for PartOfSpeech {
    type Error = UnknownPartOfSpeech;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match &value[..] {
            "noun" => Ok(Self::Noun),
            "pronoun" => Ok(Self::Pronoun),
            "verb" => Ok(Self::Verb),
            "adjective" => Ok(Self::Adjective),
            "adverb" => Ok(Self::Adverb),
            "preposition" => Ok(Self::Preposition),
            "conjunction" => Ok(Self::Conjunction),
            "interjection" => Ok(Self::Interjection),
            _ => Err(UnknownPartOfSpeech { kind: value }),
        }
    }
}

#[derive(Debug)]
pub struct WordDefinition {
    pub definition: String,
    pub example: Option<String>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
}

impl From<ApiWordDefinition> for WordDefinition {
    fn from(word: ApiWordDefinition) -> Self {
        Self {
            definition: word.definition,
            example: word.example,
            synonyms: word.synonyms,
            antonyms: word.antonyms,
        }
    }
}

#[derive(Debug)]
pub enum DictionaryError {
    Fetch(reqwest::Error),
    Deserialize(reqwest::Error),
    Conversion(UnknownPartOfSpeech),
    NotFound(NotFoundError),
}

#[derive(Debug)]
pub struct NotFoundError {
    message: String,
}

pub struct Dictionary {
    client: reqwest::Client,
}

impl Dictionary {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn get_definition(&self, word: &str) -> Result<Word, DictionaryError> {
        let url = format!("{URL}{word}");
        let res: reqwest::Response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(DictionaryError::Fetch)?;
        res.json::<ApiResponse>()
            .await
            .map_err(DictionaryError::Deserialize)
            .and_then(|res| match res {
                ApiResponse::Found(mut words) => words
                    .pop()
                    .unwrap()
                    .try_into()
                    .map_err(DictionaryError::Conversion),
                ApiResponse::Error(error) => Err(DictionaryError::NotFound(NotFoundError {
                    message: error.message,
                })),
            })
    }
}
