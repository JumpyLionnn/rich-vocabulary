use dictionary::{Dictionary, Word};
use questions::{generate_question_definition_word, generate_question_word_synonym, Question};
use storage::Storage;
use utilities::{input, str_to_bool};

use rand::seq::SliceRandom;
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
            let question = match generate_question_word_synonym(&storage, &dict, entry.uid, &word)
                .await
            {
                Ok(question) => question,
                Err(_) => generate_question_definition_word(&storage, &dict, entry.uid, word).await?,
            };
            ask_question(&storage, question).await?;
            storage.mark_word_as_quizzed_by_uid(entry.uid).await?;
        }
    }
    Ok(())
}

async fn ask_question(storage: &Storage, mut question: Question) -> Result<(), anyhow::Error> {
    println!("{}", question.question);
    question.answers.shuffle(&mut rand::thread_rng());
    for (index, answer) in question.answers.iter().enumerate() {
        println!("[{}]: {}", index + 1, answer.content);
    }
    let answer = loop {
        let chosen_answer = input("Enter the number of the correct answer: ")?;
        let chosen_answer = chosen_answer.trim();
        if chosen_answer == "exit" {
            break None;
        } else if let Ok(index) = chosen_answer.parse::<usize>() {
            if let Some(answer) = question.answers.get(index.wrapping_sub(1)) {
                break Some(answer);
            }
        }
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
                storage
                    .multiply_score_by_uid(word_uid, modifier)
                    .await?;
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
