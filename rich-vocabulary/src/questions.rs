use dictionary::{Dictionary, Word, WordDefinition, WordMeaning};
use rand::{seq::{IteratorRandom, SliceRandom}, Rng};

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
    pub word_uid: Option<i64>
}


pub async fn generate_question_word_synonym(
    storage: &Storage,
    dict: &Dictionary,
    word_uid: i64,
    word: &Word,
) -> Option<Question> {
    let (meaning, definitions) = word
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
        .choose(&mut rand::thread_rng())?;
    let answer_count = 4;
    let mut answers = Vec::with_capacity(answer_count);
    let synonym = meaning
        .synonyms
        .iter()
        .chain(definitions.synonyms.iter())
        .choose(&mut rand::thread_rng())
        .unwrap(); // there must be at least one synonym
    answers.push(Answer {
        content: synonym.clone(),
        correct: true,
        word_uid: storage.get_word(&synonym).await.unwrap().map(|word| word.uid)
    });
    let antonym = meaning
        .antonyms
        .iter()
        .chain(definitions.antonyms.iter())
        .choose(&mut rand::thread_rng())
        .unwrap(); // there must be at least one antonym
    answers.push(Answer {
        content: antonym.clone(),
        correct: false,
        word_uid: storage.get_word(&antonym).await.unwrap().map(|word| word.uid)
    });
    let mut invalid_words = word.all_synonyms()
        .chain(word.all_antonyms())
        .chain(Some(&word.word[..]))
        .collect::<Vec<&str>>();
    let words = storage
        .find_words_excluding(&invalid_words, answer_count - answers.len())
        .await
        .unwrap();
    for word in words.iter() {
        invalid_words.push(&word.word);
        answers.push(Answer {
            content: word.word.clone(),
            correct: false,
            word_uid: Some(word.uid)
        });
    }

    if answers.len() < answer_count {
        let random_words_count = answer_count - answers.len();
        let words = dict
            .get_random_words(random_words_count * 2, None)
            .await
            .unwrap()
            .into_iter()
            .filter(|word| !invalid_words.contains(&&word[..]))
            .take(random_words_count)
            .map(|word| Answer {
                content: word,
                correct: false,
                word_uid: None
            });
        answers.extend(words);
    }
    Some(Question {
        word_uid,
        question: format!("What is the synonym of {}?", word.word),
        answers,
    })
}

pub async fn generate_question_definition_word(
    storage: &Storage,
    dict: &Dictionary,
    uid: i64,
    word: Word,
) -> Result<Question, anyhow::Error> {
    // question kind: match the definition to the correct word
    let meaning: &WordMeaning = word.meanings.choose(&mut rand::thread_rng()).unwrap();
    let definition: &WordDefinition = meaning.definitions.choose(&mut rand::thread_rng()).unwrap();

    let answers_count = 4;
    let mut answers = Vec::with_capacity(answers_count);
    answers.push(Answer {
        content: word.word.clone(),
        correct: true,
        word_uid: Some(uid)
    });
    let antonym_answer = definition
        .antonyms
        .choose(&mut rand::thread_rng())
        .or_else(|| meaning.antonyms.choose(&mut rand::thread_rng()));
    if let Some(anonym) = antonym_answer {
        if rand::thread_rng().gen_bool(0.5) {
            answers.push(Answer {
                content: anonym.to_owned(),
                correct: false,
                word_uid: storage.get_word(&anonym).await?.map(|word| word.uid)
            });
        }
    }
    let min_existing_words = 1;
    let max_existing_words = answers_count - answers.len();
    let existing_words_limit =
        rand::thread_rng().gen_range(min_existing_words..=max_existing_words);
    let invalid_words = word.all_synonyms()
        .chain(answers.iter().map(|answer| &answer.content[..]))
        .collect::<Vec<&str>>();
    let words = storage
        .find_words_excluding(&invalid_words, existing_words_limit)
        .await?;
    let other_random_words_count = answers_count - words.len() - answers.len();
    if other_random_words_count > 0 {
        let mut words = dict
            .get_random_words(other_random_words_count, None)
            .await?;
        words.retain(|word| !invalid_words.contains(&&word[..]));
        for word in words {
            answers.push(Answer {
                content: word,
                correct: false,
                word_uid: None
            });
        }
    }
    for word in words.iter() {
        answers.push(Answer {
            content: word.word.clone(),
            correct: false,
            word_uid: Some(word.uid)
        });
    }

    let question = Question {
        word_uid: uid,
        question: format!(
            "What word matches the following definition? {:?}",
            definition.definition
        ),
        answers,
    };
    Ok(question)
}
