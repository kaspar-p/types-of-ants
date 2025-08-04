import { NewsletterBox } from "./NewsletterBox";
import { SuggestionBox } from "./SuggestionBox";

type InputBannerProps = {
  onSuggestion?: () => Promise<void>;
};

export default function InputBanner(props?: InputBannerProps) {
  return (
    <div className="w-full flex flex-row justify-start flex-wrap">
      <SuggestionBox onSuggestion={props?.onSuggestion} />
      <NewsletterBox />
    </div>
  );
}
