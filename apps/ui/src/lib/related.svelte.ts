/** A link of the Related column: what a page points to beside its own content. */
export type RelatedLink = { label: string; href: string };

class RelatedLinks {
  links = $state.raw<RelatedLink[]>([]);
  #owner: object | null = null;

  /** Shows a page's links until the page leaves; a later page takes the column over. */
  show(links: RelatedLink[]): () => void {
    const owner = {};
    this.#owner = owner;
    this.links = links;
    return () => {
      if (this.#owner !== owner) return;
      this.#owner = null;
      this.links = [];
    };
  }
}

export const related = new RelatedLinks();

/** Fills the Related column while the calling component lives; call it inside a component. */
export function relatedLinks(links: () => RelatedLink[]): void {
  $effect(() => related.show(links()));
}
