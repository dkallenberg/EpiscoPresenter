// The parish's own Prayers of the People and Eucharistic Prayers (library
// items), offered beside the Prayer Book's when a service is made and in
// Service settings.

import { call } from "/js/control/util.js";

/** Prayer Book forms of the Prayers of the People: [value, label]. */
export const PRAYERS_FORMS = [
  ["1", "Form I"], ["2", "Form II"], ["3", "Form III"], ["4", "Form IV"], ["5", "Form V"], ["6", "Form VI"], ["rite1", "Rite I prayer (BCP 328)"],
];

/** The Prayer Book's Eucharistic Prayers in each rite. */
export const EUCHARISTIC_PRAYERS = {
  II: [["A", "Prayer A"], ["B", "Prayer B"], ["C", "Prayer C"], ["D", "Prayer D"]],
  I: [["I", "Prayer I"], ["II", "Prayer II"]],
};

const isPrayersForm = (base) => /^(rite1\/)?prayers-of-the-people/.test(base ?? "");
const inRite = (e, rite) => !e.rite || !rite || e.rite === rite;
const label = (e) => (e.base && e.created ? `${e.title} (parish copy)` : e.title);

/** The parish's texts for a rite ("I" or "II"): { prayers, eucharistic }
 *  as [library id, label]. */
export async function parishTexts(rite) {
  const entries = (await call("library_list")) ?? [];
  const own = entries.filter((e) => !e.builtin && inRite(e, rite));
  return {
    prayers: own.filter((e) => e.uses === "prayers_of_the_people" || isPrayersForm(e.base)).map((e) => [e.id, label(e)]),
    eucharistic: own.filter((e) => e.category === "eucharistic_prayer").map((e) => [e.id, label(e)]),
  };
}

const extra = (doc, key) => doc?.meta?.extra?.find(([k]) => k === key)?.[1];

/** What the open service has now: { prayers, eucharistic } as the values
 *  the choices use (a form, a prayer's letter, a library id by title, or
 *  "" for the dialogue and Sanctus only). */
export function currentChoices(service, texts) {
  const docOf = (c) => c?.content?.document;
  const byTitle = (list, c) => list.find(([, l]) => l === c?.title || l === `${c?.title} (parish copy)`)?.[0];
  const prayersCue = service?.cues.find((c) => {
    const d = docOf(c);
    return isPrayersForm(extra(d, "based_on")) || extra(d, "uses") === "prayers_of_the_people";
  });
  let prayers = "";
  if (prayersCue) {
    const base = extra(docOf(prayersCue), "based_on");
    const own = extra(docOf(prayersCue), "created") || !base;
    prayers = own ? byTitle(texts.prayers, prayersCue) ?? "" : base === "rite1/prayers-of-the-people" ? "rite1" : base.replace("prayers-of-the-people-", "");
  }
  // (The dialogue alone is marked a Eucharistic Prayer too.)
  const ep = service?.cues.find((c) => c.category === "eucharistic_prayer" && !extra(docOf(c), "based_on")?.endsWith("sursum-corda"));
  let eucharistic = "";
  if (ep) {
    const base = extra(docOf(ep), "based_on");
    const own = extra(docOf(ep), "created") || !base;
    const letter = { "eucharistic-prayer-a": "A", "eucharistic-prayer-b": "B", "eucharistic-prayer-c": "C", "eucharistic-prayer-d": "D", "rite1/eucharistic-prayer-1": "I", "rite1/eucharistic-prayer-2": "II" }[base];
    eucharistic = own ? byTitle(texts.eucharistic, ep) ?? "" : letter ?? "";
  }
  return { prayers, eucharistic, hasPrayers: Boolean(prayersCue), hasThanksgiving: Boolean(ep) || service?.cues.some((c) => extra(docOf(c), "based_on")?.endsWith("sursum-corda")) };
}

export const riteOf = (service) => (service?.rite === "I" ? "I" : "II");
