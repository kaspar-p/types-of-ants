"use client";

import { ChangePasswordsBox } from "@/components/ChangePasswordsBox";
import { useTimedText } from "@/components/useTimedText";
import {
  password,
  passwordResetCode,
  passwordResetSecret,
} from "@/server/posts";
import Link from "next/link";
import { FormEvent, useState } from "react";

export default function Page() {
  const [step, setStep] = useState<0 | 1 | 2 | 3>(0);

  const [username, setUsername] = useState("");
  const [usernameValidationMsg, setUsernameValidationMsg] = useState("");
  const [phoneNumber, setPhoneNumber] = useState("");
  const [phoneNumberValidationMsg, setPhoneNumberValidationMsg] = useState("");
  const [codeRequestValidationMsg, setCodeRequestValidationMsg] = useState<{
    valid: boolean;
    msg: string;
  }>({ valid: false, msg: "" });

  const [otp, setOtp] = useState("");
  const [secretRequestValidationMsg, setSecretRequestValidationMsg] = useState<{
    valid: boolean;
    msg: string;
  }>({ valid: false, msg: "" });

  const [secret, setSecret] = useState("");
  const [password1, setPassword1] = useState("");
  const [password2, setPassword2] = useState("");
  const [passwordValidationMsg, setPasswordValidationMsg] = useState<{
    valid: boolean;
    msg: string;
  }>({ valid: false, msg: "" });

  async function handleCodeRequest(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();

    const res = await passwordResetCode({ username, phoneNumber });
    switch (res.__status) {
      case 200: {
        setCodeRequestValidationMsg({
          valid: true,
          msg: "one-time code sent!",
        });
        setStep(1);
        break;
      }
      case 400: {
        for (const err of res.errors) {
          switch (err.field) {
            case "phoneNumber": {
              setPhoneNumberValidationMsg(err.msg.toLocaleLowerCase());
              break;
            }
            case "username": {
              setUsernameValidationMsg(err.msg.toLocaleLowerCase());
              break;
            }
          }
        }
        break;
      }
      default: {
        setCodeRequestValidationMsg({
          valid: false,
          msg: "something went wrong, please retry",
        });
        break;
      }
    }
  }

  async function handleSecretRequest(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();

    const res = await passwordResetSecret({ phoneNumber, otp });
    switch (res.__status) {
      case 200: {
        setSecret(res.secret);
        setSecretRequestValidationMsg({
          valid: true,
          msg: "one-time code valid!",
        });
        setStep(2);
        break;
      }
      case 400: {
        setSecretRequestValidationMsg({ valid: false, msg: "invalid code." });
        break;
      }
      default: {
        setSecretRequestValidationMsg({
          valid: false,
          msg: "something went wrong, please retry",
        });
        break;
      }
    }
  }

  return (
    <div className="h-full w-full flex flex-col md:flex-row justify-center">
      <div className="m-4 w-full md:w-8/12 xl:w-3/12">
        <h2>reset your password</h2>
        <div>
          enter your details to reset your password. you will receive a one-time
          code.
        </div>

        <form autoComplete="off" onSubmit={(e) => handleCodeRequest(e)}>
          <div className="grid grid-cols-3 gap-0">
            <span className="flex flex-col justify-center">your username:</span>
            <input
              className="m-1"
              type="text"
              name="username"
              autoComplete="off"
              placeholder="kaspar"
              value={username}
              onChange={(e) => {
                setUsername(e.target.value);
                setUsernameValidationMsg("");
              }}
            />
            <span
              className={`flex flex-col justify-center m-1 text-red-600 content-center`}
            >
              {usernameValidationMsg}
            </span>

            <span className="flex flex-col justify-center">
              your phone number:
            </span>
            <input
              className="m-1"
              type="text"
              name="phoneNumber"
              autoComplete="off"
              placeholder="+1 (000) 111-2222"
              value={phoneNumber}
              onChange={(e) => {
                setPhoneNumber(e.target.value);
                setPhoneNumberValidationMsg("");
              }}
            />
            <span
              className={`flex flex-col justify-center m-1 text-red-600 content-center`}
            >
              {phoneNumberValidationMsg}
            </span>
          </div>

          <div className="w-8/12">
            <input type="submit" value="send one-time code" />
            <span
              className={`flex flex-col justify-center m-1 text-${
                codeRequestValidationMsg.valid ? "green" : "red"
              }-600 content-center`}
            >
              {codeRequestValidationMsg.msg}
            </span>
          </div>
        </form>

        {step >= 1 && (
          <>
            <h2>submit your one-time code</h2>
            <div>enter the one-time code sent to your phone.</div>

            <form autoComplete="off" onSubmit={(e) => handleSecretRequest(e)}>
              <div className="grid grid-cols-3 gap-0">
                <span className="flex flex-col justify-center">
                  one-time code:
                </span>
                <input
                  className="m-1"
                  type="text"
                  name="otp"
                  autoComplete="off"
                  placeholder=""
                  value={otp}
                  onChange={(e) => {
                    setOtp(e.target.value);
                    setSecretRequestValidationMsg({ valid: false, msg: "" });
                  }}
                />
                <span
                  className={`flex flex-col justify-center m-1 text-red-600 content-center`}
                >
                  {" "}
                </span>
              </div>
              <div className="w-8/12">
                <input type="submit" value="submit one-time code" />
                <span
                  className={`flex flex-col justify-center m-1 text-${
                    secretRequestValidationMsg.valid ? "green" : "red"
                  }-600 content-center`}
                >
                  {secretRequestValidationMsg.msg}
                </span>
              </div>
            </form>
          </>
        )}

        {step >= 2 && (
          <ChangePasswordsBox secret={secret} onValid={() => setStep(3)} />
        )}

        {step >= 3 && <Link href="/login">go back and log in</Link>}
      </div>
    </div>
  );
}
