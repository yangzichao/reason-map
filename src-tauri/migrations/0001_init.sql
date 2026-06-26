-- reason-map schema v1.
-- Source of truth = state tables (SPEC §5). `events` is an append-only history/undo log,
-- NOT the source of truth. IDs are app-generated ULIDs (TEXT). Timestamps are ISO-8601 TEXT.
-- WAL + foreign_keys + busy_timeout are set per-connection (see db::connect), not here,
-- because PRAGMA journal_mode cannot run inside the migration transaction.

-- A document. One argument map = one row (exported as one .argmap.json when shared).
CREATE TABLE maps (
    id          TEXT PRIMARY KEY NOT NULL,
    title       TEXT NOT NULL,
    meta        TEXT NOT NULL DEFAULT '{}',     -- JSON: free-form per-map settings
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    deleted_at  TEXT                            -- soft delete (trash / restore)
);

-- A node = a claim/proposition, with status + provenance. NOT a bare text box.
CREATE TABLE nodes (
    id          TEXT PRIMARY KEY NOT NULL,
    map_id      TEXT NOT NULL REFERENCES maps(id) ON DELETE CASCADE,
    text        TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'open'
                  CHECK (status IN ('fact','assumption','bet','evidenced','open')),
    origin      TEXT NOT NULL DEFAULT 'user'
                  CHECK (origin IN ('user','ai_suggested','ai_accepted')),
    x           REAL NOT NULL DEFAULT 0,
    y           REAL NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    deleted_at  TEXT
);
CREATE INDEX idx_nodes_map ON nodes(map_id) WHERE deleted_at IS NULL;

-- A typed reasoning relation. Strength is OPTIONAL (SPEC §3: no forced numeric confidence).
CREATE TABLE edges (
    id          TEXT PRIMARY KEY NOT NULL,
    map_id      TEXT NOT NULL REFERENCES maps(id) ON DELETE CASCADE,
    from_node   TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    to_node     TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    type        TEXT NOT NULL DEFAULT 'support'
                  CHECK (type IN ('support','rebut','premise_of','depends_on')),
    strength    TEXT                            -- NULL = no quantification; optional 'strong'/'weak'/'tentative'
                  CHECK (strength IS NULL OR strength IN ('strong','weak','tentative')),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    deleted_at  TEXT
);
CREATE INDEX idx_edges_map ON edges(map_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_edges_from ON edges(from_node);
CREATE INDEX idx_edges_to ON edges(to_node);

-- Evidence / citations attached to a node.
CREATE TABLE evidence (
    id          TEXT PRIMARY KEY NOT NULL,
    node_id     TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL
                  CHECK (kind IN ('url','quote','data','file')),
    payload     TEXT NOT NULL DEFAULT '{}',     -- JSON
    created_at  TEXT NOT NULL
);
CREATE INDEX idx_evidence_node ON evidence(node_id);

-- A challenge = an LLM adversarial attack. First-class object in the STAGING layer:
-- it is NOT a node in the argument until the user promotes it (SPEC §4.1).
CREATE TABLE challenges (
    id           TEXT PRIMARY KEY NOT NULL,
    map_id       TEXT NOT NULL REFERENCES maps(id) ON DELETE CASCADE,
    target_kind  TEXT NOT NULL CHECK (target_kind IN ('node','edge')),
    target_id    TEXT NOT NULL,                 -- node.id or edge.id (not FK: edge/node union)
    kind         TEXT NOT NULL
                   CHECK (kind IN ('rebuttal','counterexample','hidden_assumption','alternative','non_sequitur')),
    content      TEXT NOT NULL,                 -- the LLM's attack
    status       TEXT NOT NULL DEFAULT 'pending'
                   CHECK (status IN ('pending','conceded','rebutted','deferred')),
    verdict      TEXT,                          -- user verdict mirrors status once judged
    user_note    TEXT,                          -- why the user judged this way / their rebuttal
    created_at   TEXT NOT NULL,
    resolved_at  TEXT
);
CREATE INDEX idx_challenges_map ON challenges(map_id);
CREATE INDEX idx_challenges_target ON challenges(target_id);
CREATE INDEX idx_challenges_pending ON challenges(map_id) WHERE status = 'pending';

-- Context-aware chat, bound to a map. context_node_ids is a JSON array of node ids
-- (not FK-enforced — dangling refs are cleaned in app code, SPEC §5).
CREATE TABLE chat_messages (
    id               TEXT PRIMARY KEY NOT NULL,
    map_id           TEXT NOT NULL REFERENCES maps(id) ON DELETE CASCADE,
    role             TEXT NOT NULL CHECK (role IN ('user','assistant','system')),
    content          TEXT NOT NULL,
    context_node_ids TEXT NOT NULL DEFAULT '[]',
    created_at       TEXT NOT NULL
);
CREATE INDEX idx_chat_map ON chat_messages(map_id);

-- Append-only change log for undo/redo + history (state tables remain source of truth).
-- Each event carries enough to invert it (before/after in payload).
CREATE TABLE events (
    id          TEXT PRIMARY KEY NOT NULL,
    map_id      TEXT NOT NULL,
    ts          TEXT NOT NULL,
    op          TEXT NOT NULL,                  -- e.g. node.create, node.update, edge.delete
    payload     TEXT NOT NULL DEFAULT '{}'      -- JSON { before, after }
);
CREATE INDEX idx_events_map ON events(map_id, ts);

-- Application state, kept separate from document data (SPEC §5).
CREATE TABLE settings (
    key    TEXT PRIMARY KEY NOT NULL,
    value  TEXT NOT NULL
);

-- Vector store for semantic search. Lives in a regular table; the actual ANN query uses
-- the sqlite-vec extension when loaded (graceful fallback to FTS otherwise, SPEC §6).
-- `dirty = 1` means text changed and the embedding must be regenerated.
CREATE TABLE node_embeddings (
    node_id     TEXT PRIMARY KEY NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    model       TEXT NOT NULL,
    dim         INTEGER NOT NULL,
    vector      BLOB,
    dirty       INTEGER NOT NULL DEFAULT 1,
    embedded_at TEXT
);

-- Full-text search over node text (FTS5). Standalone table kept in sync via triggers,
-- because our PK is a TEXT ULID (not an integer rowid). UNINDEXED node_id for lookup.
CREATE VIRTUAL TABLE nodes_fts USING fts5(node_id UNINDEXED, text);

CREATE TRIGGER nodes_fts_ai AFTER INSERT ON nodes BEGIN
    INSERT INTO nodes_fts(node_id, text) VALUES (new.id, new.text);
END;
CREATE TRIGGER nodes_fts_ad AFTER DELETE ON nodes BEGIN
    DELETE FROM nodes_fts WHERE node_id = old.id;
END;
CREATE TRIGGER nodes_fts_au AFTER UPDATE OF text ON nodes BEGIN
    DELETE FROM nodes_fts WHERE node_id = old.id;
    INSERT INTO nodes_fts(node_id, text) VALUES (new.id, new.text);
END;
