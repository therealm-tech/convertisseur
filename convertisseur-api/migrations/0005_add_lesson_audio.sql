-- A lesson can carry a recording too, like a card.
ALTER TABLE lessons ADD COLUMN audio_id uuid UNIQUE REFERENCES audio (id);
