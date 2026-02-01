import { TwoFactorVerificationBox } from "../two-factor";

export default function TwoFactorPage() {
  return (
    <div className="h-full w-full flex flex-col md:flex-row justify-center">
      <>
        <div className="m-4 w-full md:w-8/12 xl:w-3/12">
          <h2>two-factor</h2>
          <TwoFactorVerificationBox />
        </div>
      </>
    </div>
  );
}
