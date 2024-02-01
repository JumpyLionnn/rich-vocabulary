use chrono::NaiveDateTime;
use sqlx::{migrate::MigrateDatabase, query, query_as, FromRow, Pool, Sqlite, SqlitePool};

const DB_URL: &str = "sqlite://sqlite.db";

#[derive(Debug, FromRow)]
pub struct WordEntry {
    pub uid: i64,
    pub word: String,
    pub last_quizzed: NaiveDateTime,
    pub score: i64,
}

pub struct Storage {
    pool: Pool<Sqlite>,
}

impl Storage {
    pub async fn initialize() -> sqlx::Result<Self> {
        if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
            Sqlite::create_database(DB_URL).await?;
        }
        let pool = SqlitePool::connect(DB_URL).await?;
        sqlx::migrate!().run(&pool).await?;
        Ok(Self { pool })
    }
}

impl Storage {
    pub async fn add_word(&self, word: &str) -> sqlx::Result<()> {
        query!("INSERT INTO words(word, score) VALUES(?, ?)", word, 500)
            .execute(&self.pool)
            .await
            .map(|_| ())
    }

    pub async fn select_random_by_score(&self, count: u32) -> sqlx::Result<Vec<WordEntry>> {
        query_as(
            "
            SELECT *, (-(score * ((SELECT MAX(last_quizzed) as latest_quiz FROM words) - last_quizzed)) / ABS(RANDOM() % 10) + 1) AS priority FROM words ORDER BY priority LIMIT ?;
            ",
        )
        .bind(count)
        .fetch_all(&self.pool).await
    }

    pub async fn mark_word_as_quizzed_by_uid(&self, uid: i64) -> sqlx::Result<()> {
        query!(
            "UPDATE words SET last_quizzed = CURRENT_TIMESTAMP WHERE uid = ?",
            uid
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Attempt to remove a word, returns true if the word was removed
    pub async fn remove_word(&self, word: &str) -> Result<bool, sqlx::Error> {
        let result = query!("DELETE FROM words WHERE word = ?", word).execute(&self.pool);
        let modified_count = result.await?;
        Ok(modified_count.rows_affected() > 0)
    }

    pub async fn find_words_excluding(
        &self,
        exclude: &[&str],
        max: usize,
    ) -> Result<Vec<WordEntry>, sqlx::Error> {
        let query = format!(
            "SELECT * FROM words WHERE word NOT IN (\"{}\") ORDER BY RANDOM() LIMIT {max};",
            exclude.join("\",\"")
        );
        query_as(&query).fetch_all(&self.pool).await
    }

    /// Adds a score to a word, returns if the word was modified or not
    pub async fn add_score_to_optional(
        &self,
        word: &str,
        additional_score: i32,
    ) -> Result<bool, sqlx::Error> {
        let result = query!(
            "UPDATE words SET score = score + ? WHERE word = ?",
            additional_score,
            word
        )
        .execute(&self.pool);
        let modified_count = result.await?;
        Ok(modified_count.rows_affected() > 0)
    }

    /// Adds a score to a word, returns if the word was modified or not
    pub async fn multiply_score_by_uid(&self, uid: i64, modifier: f64) -> Result<bool, sqlx::Error> {
        let round_weight = if modifier > 1.0 { 0.5 } else { -0.5 };
        let result = query!(
            "UPDATE words SET score = MIN(ROUND(score * ? + ?), 1000) WHERE uid = ?",
            modifier,
            round_weight,
            uid
        )
        .execute(&self.pool);
        let modified_count = result.await?;
        Ok(modified_count.rows_affected() > 0)
    }

    pub async fn get_word(&self, word: &str) -> Result<Option<WordEntry>, sqlx::Error> {
        query_as!(WordEntry, "SELECT * FROM words WHERE word = ?", word).fetch_optional(&self.pool).await
    }
}
