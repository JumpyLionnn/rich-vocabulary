use std::io::{self, Write};
use dictionary::Dictionary;


#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let client = Dictionary::new();

    let word = input("Enter a word: ").unwrap();

    match client.get_definition(&word).await {
        Ok(word) => {
            println!("Showing definition for '{}':", word.word);
            for meaning in word.meanings {
                println!("    {:?}:", meaning.part_of_speech);
                for definition in meaning.definitions {
                    println!("        {}", definition.definition);
                    if let Some(example) = definition.example {
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
        },
        Err(error) => {
            match error {
                dictionary::DictionaryError::NotFound(_) => {
                    println!("Couldn't find the word you were looking for.")
                },
                other => {
                    println!("Encountered an error while searching for the word definition: {other:?}");
                }
            }
        }
    }
}


fn input(prompt: &str) -> io::Result<String> {
    let mut line = String::new();
    print!("{prompt}");
    io::stdout().flush()?;
    io::stdin().read_line(&mut line)?;
    Ok(line)
}