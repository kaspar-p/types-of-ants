import { createContext, useContext } from "react";

export type User = {
  userId: string;
  username: string;
  phoneNumbers: string[];
  emails: string[];
  joined: Date;
};
export type TUserContext =
  | { weakAuth: false }
  | { weakAuth: true; loggedIn: false }
  | { weakAuth: true; loggedIn: true; user: User };

export const UserContext = createContext<
  undefined | { user: TUserContext; setUser: (user: TUserContext) => void }
>(undefined);

export const useUser = (): {
  user: TUserContext;
  setUser: (user: TUserContext) => void;
} => {
  const user = useContext(UserContext);
  if (!user) throw new Error("UserContext not initialized");

  return user;
};
