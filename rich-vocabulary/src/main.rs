use dictionary::{Dictionary, Word};
use questions::{
    generate_question_definition_word, generate_question_word_synonym, Question,
    QuestionGenerationError,
};
use storage::Storage;
use utilities::{input, str_to_bool};

use rand::{seq::SliceRandom, Rng};

use crate::questions::Answer;
mod questions;
mod storage;
mod utilities;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let storage = Storage::initialize().await?;

    let dict = Dictionary::new();
    loop {
        let word = input(">> ")?;
        let word = word.trim();
        let mut command_parts = word.split_ascii_whitespace();
        if let Some(command) = command_parts.next() {
            match command {
                "exit" | "leave" | "quit" | "e" | "q" | "l" => {
                    break;
                }
                "define" | "find" => {
                    define_word(
                        &dict,
                        &storage,
                        &command_parts.collect::<Vec<&str>>().join(" "),
                    )
                    .await?;
                }
                "remove" => {
                    let word = command_parts.collect::<Vec<&str>>().join(" ");
                    remove_word(&storage, &word).await?;
                }
                "practice" => {
                    practice(&storage, &dict).await?;
                }
                _ => {
                    println!("Unknown command {command}.");
                }
            }
        }
    }
    Ok(())
}

async fn practice(storage: &Storage, dict: &Dictionary) -> anyhow::Result<()> {
    let words = storage.select_random_by_score(4).await?;
    for (index, entry) in words.into_iter().enumerate() {
        if index != 0 {
            println!("----------------------------------------");
        }
        let word = dict.get_definition(&entry.word).await;
        if let Ok(word) = word {
            let question = generate_question(storage, dict, entry.uid, &word).await?;
            ask_question(&storage, question).await?;
            storage.mark_word_as_quizzed_by_uid(entry.uid).await?;
        }
    }
    Ok(())
}

async fn generate_question(
    storage: &Storage,
    dict: &Dictionary,
    uid: i64,
    word: &Word,
) -> Result<Question, QuestionGenerationError> {
    let question_kind = rand::thread_rng().gen_range(0..=1);
    match question_kind {
        0 => {
            let question = generate_question_word_synonym(&storage, &dict, uid, &word).await;
            match question {
                Ok(question) => Ok(question),
                Err(QuestionGenerationError::Unsupported) => {
                    generate_general_question(storage, dict, uid, word).await
                }
                Err(error) => Err(error),
            }
        }
        1 => generate_question_definition_word(&storage, &dict, uid, word).await,
        other => {
            unreachable!("There is no such question kind {other}");
        }
    }
}

async fn generate_general_question(
    storage: &Storage,
    dict: &Dictionary,
    uid: i64,
    word: &Word,
) -> Result<Question, QuestionGenerationError> {
    generate_question_definition_word(&storage, &dict, uid, word).await
}

async fn ask_question(storage: &Storage, mut question: Question) -> Result<(), anyhow::Error> {
    println!("{}", question.question);
    question.answers.shuffle(&mut rand::thread_rng());
    for (index, answer) in question.answers.iter().enumerate() {
        println!("[{}]: {}", index + 1, answer.content);
    }
    let answer = loop {
        let chosen_answer = input("Enter the correct answer: ")?;
        let chosen_answer = chosen_answer.trim();
        match chosen_answer.parse::<usize>() {
            Ok(index) => {
                if let Some(answer) = question.answers.get(index.wrapping_sub(1)) {
                    break Some(answer);
                }
            }
            Err(_) => match &chosen_answer.to_lowercase()[..] {
                ":s" | ":skip" => {
                    break None;
                }
                other => {
                    let mut answers = question
                        .answers
                        .iter()
                        .map(|answer| (answer, strsim::jaro(&answer.content.to_lowercase(), other)))
                        .collect::<Vec<(&Answer, f64)>>();
                    // most similar at the start
                    answers.sort_unstable_by(|(_, a), (_, b)| (-a).partial_cmp(&-b).unwrap());
                    let difference = f64::abs(answers[0].1 - answers[1].1);
                    if (answers[0].1 > 0.9 && difference > 0.25) || answers[0].1 == 1.0 {
                        break Some(answers[0].0);
                    }
                }
            },
        }
        println!("Couldn't understand your answer, please try again.");
    };
    if let Some(answer) = answer {
        if answer.correct {
            println!("The answer is correct. Well done!");
            storage
                .multiply_score_by_uid(question.word_uid, 0.92)
                .await?;
        } else {
            println!(
                "The answer is incorrect. The right answer is {}. ",
                question
                    .answers
                    .iter()
                    .find(|answer| answer.correct)
                    .map(|answer| &answer.content[..])
                    .unwrap_or("unknown")
            );
            let modifier = 1.04;
            storage
                .multiply_score_by_uid(question.word_uid, modifier)
                .await?;
            if let Some(word_uid) = answer.word_uid {
                storage.multiply_score_by_uid(word_uid, modifier).await?;
            }
        }
    }
    Ok(())
}

async fn define_word(dict: &Dictionary, storage: &Storage, word: &str) -> anyhow::Result<()> {
    match dict.get_definition(&word).await {
        Ok(word) => {
            let multiple_search_score = 5;
            print_definition(&word);
            let modified: bool = storage
                .add_score_to_optional(&word.word, multiple_search_score)
                .await?;
            if !modified {
                let practice = input("Would you like to practice this word? (Y/n): ").unwrap();
                let save = str_to_bool(practice).unwrap_or(false);
                if save {
                    match storage.add_word(&word.word).await {
                        Ok(_) => {
                            println!("Saved the word successfully");
                        }
                        Err(error) => {
                            eprintln!("Failed to save the word: {error}");
                        }
                    }
                }
            }
        }
        Err(error) => match error {
            dictionary::DictionaryError::NotFound(_) => {
                println!("Couldn't find the word you were looking for.")
            }
            other => {
                println!("Encountered an error while searching for the word definition: {other:?}");
            }
        },
    }
    Ok(())
}

async fn remove_word(storage: &Storage, word: &str) -> sqlx::Result<()> {
    if storage.remove_word(word).await? {
        println!("Deleted the word successfully.");
    } else {
        println!("This word is not saved.");
    }
    Ok(())
}

fn print_definition(word: &Word) {
    println!("Showing definition for '{}':", word.word);
    for meaning in &word.meanings {
        println!("    {:?}:", meaning.part_of_speech);
        for definition in &meaning.definitions {
            println!("        {}", definition.definition);
            if let Some(example) = &definition.example {
                println!("          example: {example}");
            }
            if !definition.synonyms.is_empty() {
                println!("          synonyms: {}", definition.synonyms.join(", "));
            }
            if !definition.antonyms.is_empty() {
                println!("          antonyms: {}", definition.antonyms.join(", "));
            }
        }
        if !meaning.synonyms.is_empty() {
            println!("      synonyms: {}", meaning.synonyms.join(", "));
        }
        if !meaning.antonyms.is_empty() {
            println!("      antonyms: {}", meaning.antonyms.join(", "));
        }
    }
}
