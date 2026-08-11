export function formatAuthors(authors: string[]) {
  if (!authors.length) return "Unknown authors";
  if (authors.length <= 2) return authors.join(", ");
  return `${authors[0]} et al.`;
}

export function formatDate(value: string) {
  const date = new Date(value);
  const difference = Date.now() - date.getTime();
  if (difference < 86_400_000) return "Today";
  if (difference < 172_800_000) return "Yesterday";
  return new Intl.DateTimeFormat("en", { month: "short", day: "numeric", year: date.getFullYear() === new Date().getFullYear() ? undefined : "numeric" }).format(date);
}

export function formatNumber(value: number) {
  return new Intl.NumberFormat("en").format(value);
}

