-- Fold cloud sync: one row of progress per signed-in user. Run once in the Supabase SQL editor.
create table if not exists progress (
  user_id uuid primary key references auth.users on delete cascade,
  data jsonb not null,
  updated_at timestamptz not null default now()
);
alter table progress enable row level security;
create policy "own row" on progress for all using (auth.uid() = user_id) with check (auth.uid() = user_id);

-- lets a signed-in user delete their own account from the app (Settings -> Delete my account)
create or replace function delete_me() returns void language sql security definer set search_path = public as
$$ delete from auth.users where id = auth.uid(); $$;
revoke all on function delete_me() from public, anon;
grant execute on function delete_me() to authenticated;
