DELETE FROM play.challenges
WHERE challenges.challenger_id = $1 OR challenges.challenged_id = $1
