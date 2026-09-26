-- Demo data. Every statement uses a fixed record id and UPSERT, so running
-- this more than once is a no-op rather than a duplicate.
--
-- The demo notes belong to a demo account nobody signs into. They exist so a
-- fresh clone has something on screen before you have written any code; real
-- sign-ins get their own empty account. Delete this file once you have your
-- own seed data.

UPSERT user:demo CONTENT {
    handle: "demo",
    display_name: "Demo User",
    email: NONE,
    appearance_mode: "ios",
    color_scheme: "light",
    created_at: time::now()
};

UPSERT note:welcome CONTENT {
    owner: user:demo,
    title: "Welcome to the g3 stack",
    body: "This note came from database/seed/00_demo.surql. The screen you are looking at is src/components/notes/note_detail.rs, the data comes from src/db/note.rs, and the animation that brought you here is declared on the Route enum in src/app.rs.",
    pinned: true,
    created_at: time::now(),
    updated_at: time::now()
};

UPSERT note:transitions CONTENT {
    owner: user:demo,
    title: "Try the transitions",
    body: "Opening a note pushes; the editor rises as a sheet; switching tabs cross-fades. None of that is written in the components — it is the #[transition(...)] attributes on the Route enum.",
    pinned: false,
    created_at: time::now(),
    updated_at: time::now()
};
