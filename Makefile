.PHONY: db-create db-sh run

db-create:
	sqlx db create
	sqlx migrate run

db-sh:
	sqlite3 -cmd "PRAGMA foreign_keys = ON" dev.db

run:
	cargo run --bin flashcards_render_cli
	cargo run --bin flashcards_server
