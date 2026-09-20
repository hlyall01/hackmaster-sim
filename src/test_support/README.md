These files define fixed inputs for combat and serialization tests. They are
independent of the editable fighter, NPC, and tactical catalogs in `data/`.

Tests should build their own players, NPCs, and policies, or use these fixtures.
Assert game-rule behavior and serialization round trips, not the names, order,
stats, or loadouts of saved user presets. Do not refresh these fixtures when a
user edits a character.

The 100,000-fight wall-clock benchmark is opt-in because its 800 ms limit depends
on the host and load. Run it with:

```sh
cargo test --release --lib bulk_sim_fixed_fixtures_100k_under_point_eight_seconds -- --ignored --test-threads=1
```
