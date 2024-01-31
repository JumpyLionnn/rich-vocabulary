#[derive(Debug)]
pub struct Word {
    pub word: String,
    pub phonetic: Option<String>,
    pub phonetics: Vec<Phonetic>,
    pub origin: Option<String>,
    pub meanings: Vec<WordMeaning>,
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
pub struct WordDefinition {
    pub definition: String,
    pub example: Option<String>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
}
