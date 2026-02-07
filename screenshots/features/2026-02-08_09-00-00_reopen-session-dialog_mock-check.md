# Mock Parity Check - ReopenSessionDialog

## Commands Found
- No `invoke()` calls — pure presentational component receiving callbacks via props

## Web Mode Test
- Component is an AlertDialog that requires `open={true}` from parent
- Not yet wired into any view (Tasks 5/6 will add triggers)
- No Tauri dependencies — will render correctly in web mode when triggered

## Result: PASS (no mocks needed)
