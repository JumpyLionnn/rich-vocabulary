use chrono::NaiveDateTime;
use dictionary::{Dictionary, Word, WordDefinition, WordMeaning};
use futures::{future::Lazy, stream::StreamExt};
use sqlx::{
    migrate::MigrateDatabase, query, query_as, Either, FromRow, Pool, Row, Sqlite, SqlitePool,
};
use std::{any::Any, collections::HashMap, io::{self, Write}};

use rand::seq::SliceRandom;

const DB_URL: &str = "sqlite://sqlite.db";

#[derive(Debug, FromRow)]
struct WordEntry {
    pub uid: i64,
    pub word: String,
    pub last_quizzed: NaiveDateTime,
    pub score: i64,
}

struct Question {
    word_uid: i64,
    question: String,
    answers: Vec<Answer>,
}

#[derive(Debug)]
struct Answer {
    content: String,
    correct: bool,
}

#[tokio::main]
async fn main() {
    if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
        println!("Creating database {}", DB_URL);
        match Sqlite::create_database(DB_URL).await {
            Ok(_) => println!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        println!("Database already exists");
    }

    let db = SqlitePool::connect(DB_URL).await.unwrap();

    query!(
        "CREATE TABLE IF NOT EXISTS words(
            uid INTEGER PRIMARY KEY NOT NULL,
            word VARCHAR NOT NULL,
            last_quizzed DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            score INTEGER NOT NULL
          );
          "
    )
    .execute(&db)
    .await
    .unwrap();

    let dict = Dictionary::new();
    loop {
        let word = input(">> ").unwrap();
        let word = word.trim();
        let mut command_parts = word.split_ascii_whitespace();
        if let Some(command) = command_parts.next() {
            match command {
                "exit" | "leave" | "quit" | "e" | "q" | "l" => {
                    break;
                }
                "define" | "find" => {
                    define_word(&dict, &db, &command_parts.collect::<Vec<&str>>().join(" ")).await;
                }
                "remove" => {
                    let word = command_parts.collect::<Vec<&str>>().join(" ");
                    remove_word(&db, &word).await;
                }
                "practice" => {
                    let words = select_random_by_score(&db, 4).await.unwrap();
                    for entry in words {
                        println!(
                            "{} with {} points at {}.",
                            entry.word, entry.score, entry.last_quizzed
                        );
                        let word = dict.get_definition(&entry.word).await;
                        if let Ok(word) = word {
                            // question kind: match the definition to the correct word
                            let meaning: &WordMeaning =
                                word.meanings.choose(&mut rand::thread_rng()).unwrap();
                            let definition: &WordDefinition =
                                meaning.definitions.choose(&mut rand::thread_rng()).unwrap();
                            let answer = definition
                                .antonyms
                                .choose(&mut rand::thread_rng())
                                .or_else(|| meaning.antonyms.choose(&mut rand::thread_rng()));
                            if let Some(answer) = answer {
                                let question = Question {
                                    word_uid: entry.uid,
                                    question: format!(
                                        "What word matches the following definition? {:?}",
                                        definition.definition
                                    ),
                                    answers: vec![
                                        Answer {
                                            content: word.word,
                                            correct: true,
                                        },
                                        Answer {
                                            content: answer.clone(),
                                            correct: false,
                                        },
                                    ],
                                };

                                ask_question(&db, question).await;
                            }
                        }
                    }
                }
                _ => {
                    println!("Unknown command {command}.");
                }
            }
        }
    }
}

async fn ask_question(db: &Pool<Sqlite>, mut question: Question) {
    println!("{}", question.question);
    question.answers.shuffle(&mut rand::thread_rng());
    for (index, answer) in question.answers.iter().enumerate() {
        println!("[{}]: {}", index + 1, answer.content);
    }
    let answer = loop {
        let chosen_answer = input("Enter the number of the correct answer: ").unwrap();
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
            query!(
                "UPDATE words SET score = ROUND(score * 0.92 - 0.5) WHERE uid = ?",
                question.word_uid
            )
            .execute(db)
            .await
            .unwrap();
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
            query!(
                "UPDATE words SET score = MIN(ROUND(score * 1.04 + 0.5), 1000) WHERE uid = ?",
                question.word_uid
            )
            .execute(db)
            .await
            .unwrap();
        }
    }
}

async fn define_word(dict: &Dictionary, db: &Pool<Sqlite>, word: &str) {
    match dict.get_definition(&word).await {
        Ok(word) => {
            let multiple_search_score = 5;
            print_definition(&word);
            let mut results = query!(
                "UPDATE words SET score = score + ? WHERE word = ?",
                multiple_search_score,
                word.word
            )
            .fetch_many(db);
            let modified_count = results
                .next()
                .await
                .map(|either| match either.unwrap() {
                    Either::Left(res) => res.rows_affected(),
                    Either::Right(row) => row.len() as u64,
                })
                .unwrap_or(0);
            if modified_count == 0 {
                let practice = input("Would you like to practice this word? (Y/n): ").unwrap();
                let save = str_to_bool(practice).unwrap_or(false);
                if save {
                    let res = query!(
                        "INSERT INTO words(word, score) VALUES(?, ?)",
                        word.word,
                        500
                    )
                    .execute(db)
                    .await;
                    match res {
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
}

async fn remove_word(db: &Pool<Sqlite>, word: &str) {
    let mut results = query!("DELETE FROM words WHERE word = ?", word).fetch_many(db);
    let modified_count = results
        .next()
        .await
        .map(|either| match either.unwrap() {
            Either::Left(res) => res.rows_affected(),
            Either::Right(row) => row.len() as u64,
        })
        .unwrap_or(0);
    if modified_count > 0 {
        println!("Deleted the word successfully.");
    } else {
        println!("This word is not saved.");
    }
}

fn input(prompt: &str) -> io::Result<String> {
    let mut line = String::new();
    print!("{prompt}");
    io::stdout().flush()?;
    io::stdin().read_line(&mut line)?;
    Ok(line)
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

fn str_to_bool(mut str: String) -> Option<bool> {
    str.make_ascii_lowercase();
    match str.trim() {
        "y" | "yes" | "yeah" | "yea" | "true" | "on" => Some(true),
        "n" | "no" | "nope" | "false" | "off" => Some(false),
        _ => None,
    }
}

async fn select_random_by_score(db: &Pool<Sqlite>, count: u32) -> sqlx::Result<Vec<WordEntry>> {
    query_as(
        "
        -- DECLARE @latest_quiz AS VARCHAR(100)=SELECT MAX(last_quizzed) FROM words;
        -- WITH vars AS (SELECT MAX(last_quizzed) as latest_quiz FROM words);
        SELECT *, (-(score * ((SELECT MAX(last_quizzed) as latest_quiz FROM words) - last_quizzed)) / ABS(RANDOM() % 10) + 1) AS priority FROM words ORDER BY priority LIMIT ?;
        ",
    )
    .bind(count)
    .fetch_all(db).await
}
