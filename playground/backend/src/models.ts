import type { Db, Collection, ObjectId } from "mongodb";

export interface Playground {
  _id?: ObjectId;
  shortId: string;
  code: string;
  encoding: "gzip" | "none";
  permanent: boolean;
  createdAt: Date;
  expiresAt: Date | null;
}

export interface CreatePlaygroundRequest {
  code: string;
  permanent?: boolean;
  encoding?: "gzip" | "none";
}

export interface CreatePlaygroundResponse {
  url: string;
  shortId: string;
}

export function getCollection(db: Db): Collection<Playground> {
  return db.collection<Playground>("playgrounds");
}
