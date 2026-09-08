export function download(text: string, filename: string) {
  const url = URL.createObjectURL(new Blob([text], { type: 'text/plain;charset=utf-8' }));
  const anchor = document.createElement('a');
  try {
    anchor.href = url;
    anchor.download = filename;
    document.body.append(anchor);
    anchor.click();
  } finally {
    anchor.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
}

export function focusError(key: string) {
  const element = document.getElementById(`cg-${key}`);
  const details = element?.closest('details');
  if (details) details.open = true;
  element?.focus();
}
