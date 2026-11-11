/**
 * A `className` from the classes that apply, dropping the ones that do not.
 *
 * Each class is still written out as a literal at its call site, which is what
 * lets `cssConventions.test.ts` see that it is used.
 */
export function cx(...classes: (string | false | null | undefined)[]): string | undefined {
  const joined = classes.filter(Boolean).join(" ");
  return joined || undefined;
}
