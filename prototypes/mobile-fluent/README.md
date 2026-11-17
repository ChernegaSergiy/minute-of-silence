# Mobile UI prototype (Fluent UI v9)

Visual-only prototype of the existing four tabs (Settings, Personal dates, About,
Changelog) in a portrait phone layout. Strings come from `src/locales/uk.json`,
the changelog from `CHANGELOG.yaml`; components and icons are the same
`@fluentui/react-components` / `@fluentui/react-icons` packages the app uses.
No Tauri/IPC: all state is local and nothing is persisted.

    npm run proto:mobile
