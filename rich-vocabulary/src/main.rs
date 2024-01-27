use std::io::{self, Write};
use dictionary::Dictionary;


#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let client = Dictionary::new();

    let mut word = String::new();
    print!("Enter a word: ");
    io::stdout().flush().expect("Couldn't flush stdout.");
    io::stdin().read_line(&mut word).expect("Couldn't read from stdout");

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
            println!("Encountered an error while searching for the word definition: {error:?}");
        }
    }
}
