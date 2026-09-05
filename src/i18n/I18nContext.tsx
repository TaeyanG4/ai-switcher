import React, { createContext, useContext, useEffect, useState } from "react";
import { Language, TranslationKey, translations } from "./translations";
import { getAppSettings, saveAppSettings } from "../api";

interface I18nContextType {
  language: Language;
  setLanguage: (lang: Language) => Promise<void>;
  t: (key: TranslationKey, params?: Record<string, string | number>) => string;
}

const I18nContext = createContext<I18nContextType>({
  language: "en",
  setLanguage: async () => {},
  t: (key) => key,
});

export const I18nProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [language, setLanguageState] = useState<Language>("en");

  useEffect(() => {
    getAppSettings()
      .then((settings) => {
        if (settings.language && (settings.language in translations)) {
          setLanguageState(settings.language as Language);
        }
      })
      .catch((err) => console.error("Failed to load language setting:", err));
  }, []);

  const setLanguage = async (newLang: Language) => {
    setLanguageState(newLang);
    try {
      const current = await getAppSettings();
      await saveAppSettings({
        ...current,
        language: newLang,
      });
    } catch (err) {
      console.error("Failed to persist language setting:", err);
    }
  };

  const t = (key: TranslationKey, params?: Record<string, string | number>): string => {
    const langDict = translations[language] as Record<string, string>;
    const enDict = translations.en as Record<string, string>;
    let template = langDict?.[key] || enDict?.[key] || key;

    if (params) {
      for (const [pKey, pVal] of Object.entries(params)) {
        template = template.replace(new RegExp(`\\{${pKey}\\}`, "g"), String(pVal));
      }
    }

    return template;
  };

  return (
    <I18nContext.Provider value={{ language, setLanguage, t }}>
      {children}
    </I18nContext.Provider>
  );
};

export const useI18n = (): I18nContextType => {
  return useContext(I18nContext);
};
