import { password } from "@/server/posts";
import { useState, FormEvent } from "react";

export type ChangePasswordsBoxProps = {
  secret: string;
  onValid: () => void;
};

export function ChangePasswordsBox(props: ChangePasswordsBoxProps) {
  const [password1, setPassword1] = useState("");
  const [password2, setPassword2] = useState("");
  const [passwordValidationMsg, setPasswordValidationMsg] = useState<{
    valid: boolean;
    msg: string;
  }>({ valid: false, msg: "" });

  async function handleNewPasswords(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();

    const res = await password({ secret: props.secret, password1, password2 });

    switch (res.__status) {
      case 400: {
        setPasswordValidationMsg({
          valid: false,
          msg: res.errors[0].msg.toLocaleLowerCase(),
        });

        break;
      }
      case 200: {
        setPasswordValidationMsg({ valid: true, msg: "password changed!" });
        setPassword1("");
        setPassword2("");
        props.onValid();
        break;
      }
    }
  }

  return (
    <>
      <h2>enter new password</h2>
      <div>change the password for your account</div>

      <form autoComplete="off" onSubmit={(e) => handleNewPasswords(e)}>
        <div className="grid grid-cols-3 gap-0">
          <span className="flex flex-col justify-center">password:</span>
          <input
            className="m-1"
            type="password"
            name="password1"
            autoComplete="off"
            placeholder=""
            value={password1}
            onChange={(e) => {
              setPassword1(e.target.value);
              setPasswordValidationMsg({ valid: false, msg: "" });
            }}
          />
          <span
            className={`flex flex-col justify-center m-1 text-red-600 content-center`}
          >
            {" "}
          </span>

          <span className="flex flex-col justify-center">repeat password:</span>
          <input
            className="m-1"
            type="password"
            name="password2"
            autoComplete="off"
            placeholder=""
            value={password2}
            onChange={(e) => {
              setPassword2(e.target.value);
              setPasswordValidationMsg({ valid: false, msg: "" });
            }}
          />
          <span
            className={`flex flex-col justify-center m-1 text-red-600 content-center`}
          >
            {" "}
          </span>
        </div>
        <div className="w-8/12">
          <input type="submit" value="change password" />
          <span
            className={`flex flex-col justify-center m-1 text-${
              passwordValidationMsg.valid ? "green" : "red"
            }-600 content-center`}
          >
            {passwordValidationMsg.msg}
          </span>
        </div>
      </form>
    </>
  );
}
