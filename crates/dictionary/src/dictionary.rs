#[derive(Debug)]
pub struct Word {
    pub word: String,
    pub phonetic: Option<String>,
    pub phonetics: Vec<Phonetic>,
    pub origin: Option<String>,
    pub meanings: Vec<WordMeaning>,
}

impl Word {
    pub fn all_synonyms(&self) -> impl Iterator<Item = &str> {
        self.meanings
            .iter()
            .flat_map(|meaning| {
                meaning.synonyms.iter().chain(
                    meaning
                        .definitions
                        .iter()
                        .flat_map(|definition| definition.synonyms.iter()),
                )
            })
            .map(|s| &s[..])
    }
    
    pub fn all_antonyms(&self) -> impl Iterator<Item = &str> {
        self.meanings
            .iter()
            .flat_map(|meaning| {
                meaning.antonyms.iter().chain(
                    meaning
                        .definitions
                        .iter()
                        .flat_map(|definition| definition.antonyms.iter()),
                )
            })
            .map(|s| &s[..])
    }    
}

#[derive(Debug)]
pub struct Phonetic {
    pub text: Option<String>,
    pub audio: Option<String>,
}


#[derive(Debug)]
pub struct WordMeaning {
    pub part_of_speech: PartOfSpeech,
    pub definitions: Vec<WordDefinition>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
}


#[derive(Debug, Clone, PartialEq)]
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
pub struct WordDefinition {
    pub definition: String,
    pub example: Option<String>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
}
