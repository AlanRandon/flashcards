db-create:
	sqlx db create
	sqlx migrate run

db-sh:
	sqlite3 -cmd "PRAGMA foreign_keys = ON" dev.db
