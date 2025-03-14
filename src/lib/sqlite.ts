import Database from "@tauri-apps/plugin-sql";

async function initializeDB() {
  const db = await Database.load("sqlite:hv-database.db");
  return {
    load: async <T>(sql: string): Promise<T[]> => {
      return db.select<T[]>(sql).catch((err) => {
        console.error(err);
        return [];
      });
    },
    save: async (sql: string): Promise<void> => {
      await db.execute(sql).catch((err) => {
        console.error(err);
      });
    },
  };
}

export const sqlitePromise = initializeDB();
