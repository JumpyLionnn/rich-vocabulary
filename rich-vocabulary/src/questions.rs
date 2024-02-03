use std::{error::Error, fmt::Display};

use dictionary::{Dictionary, DictionaryError, PartOfSpeech, Word, WordDefinition, WordMeaning};
use rand::{
    seq::{IteratorRandom, SliceRandom},
    Rng,
};

use crate::storage::Storage;

#[derive(Debug, Clone)]
pub struct Question {
    pub word_uid: i64,
    pub question: String,
    pub answers: Vec<Answer>,
}

#[derive(Debug, Clone)]
pub struct Answer {
    pub content: String,
    pub correct: bool,
    pub word_uid: Option<i64>,
}

#[derive(Debug)]
pub enum QuestionGenerationError {
    /// This error kind means that the current format is not supported for this word
    /// It usually happen when the word lacks synonyms and antonyms
    Unsupported,
    Storage(sqlx::Error),
    Dictionary(DictionaryError),
}

impl Error for QuestionGenerationError {}
impl Display for QuestionGenerationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuestionGenerationError::Unsupported => f.write_str("Unsupported question kind"),
            QuestionGenerationError::Storage(error) => f.write_fmt(format_args!("{error}")),
            QuestionGenerationError::Dictionary(error) => f.write_fmt(format_args!("{error}")),
        }
    }
}

/// If synonym is true the correct answer will be a synonym otherwise its gonna be an antonym
pub async fn generate_question_word_synonym(
    storage: &Storage,
    dict: &Dictionary,
    word_uid: i64,
    word: &Word,
    is_synonym: bool,
) -> Result<Question, QuestionGenerationError> {
    let (meaning, definition) = word
        .meanings
        .iter()
        .map(|meaning| {
            (
                meaning,
                meaning
                    .definitions
                    .iter()
                    .filter(|definition| {
                        (!definition.synonyms.is_empty() || !meaning.synonyms.is_empty())
                            && (!definition.antonyms.is_empty() || !meaning.definitions.is_empty())
                    })
                    .choose(&mut rand::thread_rng()),
            )
        })
        .filter_map(|(meaning, definition)| {
            if let Some(definition) = definition {
                Some((meaning, definition))
            } else {
                None
            }
        })
        .choose(&mut rand::thread_rng())
        .ok_or(QuestionGenerationError::Unsupported)?;
    let answer_count = 4;
    let mut answers = Vec::with_capacity(answer_count);
    let synonym = meaning
        .synonyms
        .iter()
        .chain(definition.synonyms.iter())
        .choose(&mut rand::thread_rng())
        .ok_or(QuestionGenerationError::Unsupported)?;
    answers.push(Answer {
        content: synonym.clone(),
        correct: is_synonym,
        word_uid: storage
            .get_word(&synonym)
            .await
            .map_err(QuestionGenerationError::Storage)?
            .map(|word| word.uid),
    });
    let antonym = meaning
        .antonyms
        .iter()
        .chain(definition.antonyms.iter())
        .choose(&mut rand::thread_rng())
        .ok_or(QuestionGenerationError::Unsupported)?;
    answers.push(Answer {
        content: antonym.clone(),
        correct: !is_synonym,
        word_uid: storage
            .get_word(&antonym)
            .await
            .map_err(QuestionGenerationError::Storage)?
            .map(|word| word.uid),
    });
    let mut invalid_words = word
        .all_synonyms()
        .chain(word.all_antonyms())
        .chain(Some(&word.word[..]))
        .collect::<Vec<&str>>();
    let existing_words_count = answer_count - answers.len();
    add_from_storage(storage, &mut answers, &invalid_words, existing_words_count)
        .await
        .map_err(QuestionGenerationError::Storage)?;

    let random_words_count = answer_count - answers.len();
    add_random(dict, &mut answers, &mut invalid_words, random_words_count)
        .await
        .map_err(QuestionGenerationError::Dictionary)?;
    let synonym_or_antonym = if is_synonym { "synonym" } else { "antonym" };
    Ok(Question {
        word_uid,
        question: format!("What is the {synonym_or_antonym} of {}?", word.word),
        answers,
    })
}

pub async fn generate_question_definition_word(
    storage: &Storage,
    dict: &Dictionary,
    uid: i64,
    word: &Word,
) -> Result<Question, QuestionGenerationError> {
    // question kind: match the definition to the correct word
    let meaning: &WordMeaning = word
        .meanings
        .choose(&mut rand::thread_rng())
        .ok_or(QuestionGenerationError::Unsupported)?;
    let definition: &WordDefinition = meaning
        .definitions
        .choose(&mut rand::thread_rng())
        .ok_or(QuestionGenerationError::Unsupported)?;

    let answers_count = 4;
    let mut answers = Vec::with_capacity(answers_count);
    let mut invalid_words = Vec::new();
    answers.push(Answer {
        content: word.word.clone(),
        correct: true,
        word_uid: Some(uid),
    });
    invalid_words.push(&word.word[..]);
    let antonym_answer = definition
        .antonyms
        .choose(&mut rand::thread_rng())
        .or_else(|| meaning.antonyms.choose(&mut rand::thread_rng()));
    if let Some(anonym) = antonym_answer {
        if rand::thread_rng().gen_bool(0.6) {
            answers.push(Answer {
                content: anonym.to_owned(),
                correct: false,
                word_uid: storage
                    .get_word(&anonym)
                    .await
                    .map_err(QuestionGenerationError::Storage)?
                    .map(|word| word.uid),
            });
            invalid_words.push(anonym);
        }
    }
    invalid_words.extend(word.all_synonyms());

    let max_existing_words = answers_count - answers.len();
    let existing_words_limit = rand::thread_rng().gen_range(1..=max_existing_words);
    add_from_storage(storage, &mut answers, &invalid_words, existing_words_limit)
        .await
        .map_err(QuestionGenerationError::Storage)?;

    let random_words_count = answers_count - answers.len();
    add_random(dict, &mut answers, &invalid_words, random_words_count)
        .await
        .map_err(QuestionGenerationError::Dictionary)?;

    Ok(Question {
        word_uid: uid,
        question: format!(
            "What word matches the following definition? {:?}",
            definition.definition
        ),
        answers,
    })
}

pub async fn generate_question_word_definition(
    storage: &Storage,
    dict: &Dictionary,
    uid: i64,
    word: &Word,
) -> Result<Question, QuestionGenerationError> {
    // question kind: match the definition to the correct word
    let meaning: &WordMeaning = word
        .meanings
        .choose(&mut rand::thread_rng())
        .ok_or(QuestionGenerationError::Unsupported)?;
    let definition: &WordDefinition = meaning
        .definitions
        .choose(&mut rand::thread_rng())
        .ok_or(QuestionGenerationError::Unsupported)?;

    let answers_count = 4;
    let mut answers = Vec::with_capacity(answers_count);
    let mut invalid_words = Vec::new();
    answers.push(Answer {
        content: definition.definition.clone(),
        correct: true,
        word_uid: None,
    });
    invalid_words.push(&word.word[..]);
    // let antonym_answer = definition
    //     .antonyms
    //     .choose(&mut rand::thread_rng())
    //     .or_else(|| meaning.antonyms.choose(&mut rand::thread_rng()));
    while let Some(anonym) = definition
        .antonyms
        .choose(&mut rand::thread_rng())
        .or_else(|| meaning.antonyms.choose(&mut rand::thread_rng()))
    {
        if rand::thread_rng().gen_bool(0.8) {
            match dict.get_definition(&anonym).await {
                Ok(word) => {
                    let antonym_meaning = word
                        .meanings
                        .iter()
                        .find(|antonym| antonym.part_of_speech == meaning.part_of_speech);
                    match antonym_meaning {
                        Some(antonym_meaning) => {
                            answers.push(Answer {
                                content: antonym_meaning
                                    .definitions
                                    .choose(&mut rand::thread_rng())
                                    .unwrap()
                                    .definition
                                    .clone(),
                                correct: false,
                                word_uid: None,
                            });
                            invalid_words.push(anonym);
                            break;
                        }
                        None => continue,
                    }
                }
                Err(DictionaryError::NotFound(_)) => continue,
                Err(error) => Err(QuestionGenerationError::Dictionary(error))?,
            }
        }
    }
    invalid_words.extend(word.all_synonyms().chain(word.all_antonyms()));

    let max_existing_words = answers_count - answers.len();
    let existing_words_limit = rand::thread_rng().gen_range(1..=max_existing_words);
    add_definitions_from_storage(
        storage,
        dict,
        &mut answers,
        &invalid_words,
        existing_words_limit,
        meaning.part_of_speech.clone(),
    )
    .await
    .map_err(QuestionGenerationError::Storage)?;

    let random_words_count = answers_count - answers.len();
    add_random_definitions(
        dict,
        &mut answers,
        &invalid_words,
        random_words_count,
        meaning.part_of_speech.clone(),
    )
    .await
    .map_err(QuestionGenerationError::Dictionary)?;

    Ok(Question {
        word_uid: uid,
        question: format!("The definition of '{}' is:", word.word),
        answers,
    })
}

async fn add_from_storage(
    storage: &Storage,
    answers: &mut Vec<Answer>,
    invalid_words: &Vec<&str>,
    count: usize,
) -> sqlx::Result<()> {
    let words = storage.find_words_excluding(&invalid_words, count).await?;
    for word in words.iter() {
        answers.push(Answer {
            content: word.word.clone(),
            correct: false,
            word_uid: Some(word.uid),
        });
    }
    Ok(())
}

async fn add_definitions_from_storage(
    storage: &Storage,
    dict: &Dictionary,
    answers: &mut Vec<Answer>,
    invalid_words: &Vec<&str>,
    mut count: usize,
    part_of_speech: PartOfSpeech,
) -> sqlx::Result<()> {
    let words = storage
        .find_words_excluding(&invalid_words, count * 2)
        .await?;
    for word in words.iter() {
        if let Ok(definition) = dict.get_definition(&word.word).await {
            if let Some(definition) = definition
                .meanings
                .into_iter()
                .find(|meaning| meaning.part_of_speech == part_of_speech)
            {
                answers.push(Answer {
                    content: definition
                        .definitions
                        .choose(&mut rand::thread_rng())
                        .unwrap()
                        .definition
                        .to_owned(),
                    correct: false,
                    word_uid: None,
                });
                count -= 1;
                if count <= 0 {
                    break;
                }
            }
        }
    }
    Ok(())
}

async fn add_random(
    dict: &Dictionary,
    answers: &mut Vec<Answer>,
    invalid_words: &Vec<&str>,
    count: usize,
) -> Result<(), DictionaryError> {
    if count > 0 {
        let words = dict
            .get_random_words(count * 2, None)
            .await?
            .into_iter()
            .filter(|word| !invalid_words.contains(&&word[..]))
            .take(count)
            .map(|word| Answer {
                content: word,
                correct: false,
                word_uid: None,
            });
        answers.extend(words);
    }
    Ok(())
}

async fn add_random_definitions(
    dict: &Dictionary,
    answers: &mut Vec<Answer>,
    invalid_words: &Vec<&str>,
    count: usize,
    part_of_speech: PartOfSpeech,
) -> Result<(), DictionaryError> {
    let words = dict
        .get_random_words(count * 3, None)
        .await?
        .into_iter()
        .filter(|word| !invalid_words.contains(&&word[..]));
    let mut added = 0;
    for word in words {
        if let Ok(definition) = dict.get_definition(&word).await {
            if let Some(definition) = definition
                .meanings
                .into_iter()
                .find(|meaning| meaning.part_of_speech == part_of_speech)
            {
                answers.push(Answer {
                    content: definition
                        .definitions
                        .choose(&mut rand::thread_rng())
                        .unwrap()
                        .definition
                        .to_owned(),
                    correct: false,
                    word_uid: None,
                });
                if added >= count {
                    break;
                }
                added += 1;
            }
        }
    }
    Ok(())
}
