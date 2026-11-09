-- The game's moves in UCI, in the order they were played.
SELECT game_moves.uci
FROM play.game_moves
WHERE game_moves.game_id = $1
ORDER BY game_moves.ply
