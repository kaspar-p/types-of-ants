"use client";

import { useUser } from "@/app/UserProvider";
import { changeUsername } from "@/server/posts";
import { FormEvent, useState } from "react";

type ChangeUsernameBoxProps = {
  onSuccess: () => void | Promise<void>;
};

export default function ChangeUsernameBox(props: ChangeUsernameBoxProps) {
  const { user, resetUser } = useUser();
  const [username, setUsername] = useState<string>("");
  const [usernameValidationMsg, setUsernameValidationMsg] = useState<{
    valid: boolean;
    msg: string;
  }>({ valid: false, msg: "" });

  async function handleNewUsername(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();

    const res = await changeUsername({ username });
    switch (res.__status) {
      case 500: {
        setUsernameValidationMsg({
          valid: false,
          msg: "something went wrong, please retry",
        });

        break;
      }

      case 409: {
        setUsernameValidationMsg({
          valid: false,
          msg: "username already taken!",
        });
        break;
      }

      case 400: {
        setUsernameValidationMsg({
          valid: false,
          msg: res.errors[0].msg.toLocaleLowerCase(),
        });

        break;
      }

      case 200: {
        setUsernameValidationMsg({ valid: true, msg: "username changed!" });
        setUsername("");

        resetUser();

        await props.onSuccess();
      }
    }
  }

  return (
    <>
      <h2>enter new username</h2>
      <div>change the username of your account</div>
      <form autoComplete="off" onSubmit={(e) => handleNewUsername(e)}>
        <div className="grid grid-cols-3 gap-0">
          <span className="flex flex-col justify-center">username:</span>
          <input
            className="m-1"
            type="text"
            name="username"
            autoComplete="off"
            placeholder=""
            value={username}
            onChange={(e) => {
              setUsername(e.target.value);
              setUsernameValidationMsg({ valid: false, msg: "" });
            }}
          />
          <span
            className={`flex flex-col justify-center m-1 text-red-600 content-center`}
          >
            {" "}
          </span>
        </div>
        <div className="w-8/12">
          <input type="submit" value="change username" />
          <span
            className={`flex flex-col justify-center m-1 text-${
              usernameValidationMsg.valid ? "green" : "red"
            }-600 content-center`}
          >
            {usernameValidationMsg.msg}
          </span>
        </div>
      </form>
    </>
  );
}
