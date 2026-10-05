import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import {
  LANGUAGE_STORAGE_KEY,
  storedLocale,
  translate,
  type Locale,
  type TextValues,
} from "./i18n-core";
export { translate, type Locale, type TextValues } from "./i18n-core";

type Language = {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: (text: string, values?: TextValues) => string;
};
const LanguageContext = createContext<Language>({
  locale: "zh-CN",
  setLocale: () => {},
  t: (text, values) => translate("zh-CN", text, values),
});

export function LanguageProvider({ children }: { children: ReactNode }) {
  const [locale, setLocale] = useState<Locale>(() => {
    try {
      return storedLocale(window.localStorage);
    } catch {
      return "zh-CN";
    }
  });
  useEffect(() => {
    document.documentElement.lang = locale;
    try {
      window.localStorage.setItem(LANGUAGE_STORAGE_KEY, locale);
    } catch {
      /* Session switching still works when storage is unavailable. */
    }
  }, [locale]);
  const value = useMemo(
    () => ({
      locale,
      setLocale,
      t: (text: string, values?: TextValues) => translate(locale, text, values),
    }),
    [locale],
  );
  return (
    <LanguageContext.Provider value={value}>
      {children}
    </LanguageContext.Provider>
  );
}

export function useLanguage() {
  return useContext(LanguageContext);
}

export function LanguagePicker() {
  const { locale, setLocale } = useLanguage();
  return (
    <label className="language-picker">
      <span>语言 / Language</span>
      <select
        aria-label="语言 / Language"
        value={locale}
        onChange={(e) => setLocale(e.target.value as Locale)}
      >
        <option value="zh-CN">简体中文</option>
        <option value="en">English</option>
      </select>
    </label>
  );
}

export function SourceMessage({
  text,
  values,
  error = false,
  progress = false,
}: {
  text: string;
  values?: TextValues;
  error?: boolean;
  progress?: boolean;
}) {
  const { locale, t } = useLanguage();
  const translated = t(text, values);
  if (locale !== "en" || t(text) !== text || !/[\u4e00-\u9fff]/.test(text))
    return <>{translated}</>;
  return (
    <span className="source-message">
      {t(
        error
          ? "操作未完成，请查看原始详情。"
          : progress
            ? "正在准备，请查看进度详情。"
            : "游戏文件提示",
      )}
      <details>
        <summary>{t("原始详情")}</summary>
        {text}
      </details>
    </span>
  );
}
