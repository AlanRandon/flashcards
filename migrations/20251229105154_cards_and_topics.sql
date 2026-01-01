CREATE TABLE topic (
	hash INTEGER PRIMARY KEY,
	name TEXT NOT NULL,
	parent INTEGER,
	cards_hash INTEGER NOT NULL,
	FOREIGN KEY (parent) REFERENCES topic (hash),
	UNIQUE (name, parent)
);

CREATE TABLE render_format (
	name TEXT NOT NULL PRIMARY KEY
);

INSERT INTO render_format (name) VALUES ("markdown");
INSERT INTO render_format (name) VALUES ("tex");
INSERT INTO render_format (name) VALUES ("typst");

CREATE TABLE rendered (
	hash INTEGER PRIMARY KEY,
	render_format TEXT NOT NULL,
	source TEXT NOT NULL,	
	html TEXT NOT NULL,
	FOREIGN KEY (render_format) REFERENCES render_format (name)
);

CREATE TABLE card (
	hash INTEGER PRIMARY KEY,
	term INTEGER NOT NULL,
	definition INTEGER NOT NULL,
	source_path TEXT NOT NULL,
	compiled_at DATETIME NOT NULL,
	FOREIGN KEY (term) REFERENCES rendered (hash),
	FOREIGN KEY (definition) REFERENCES rendered (hash)
);

CREATE TABLE card_topic (
	card INTEGER NOT NULL,
	topic TEXT NOT NULL,
	FOREIGN KEY (card) REFERENCES card (hash) ON DELETE CASCADE,
	FOREIGN KEY (topic) REFERENCES topic (hash) ON DELETE CASCADE,
	PRIMARY KEY (card, topic)
);
