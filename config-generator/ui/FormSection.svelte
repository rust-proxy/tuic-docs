<script lang="ts">
  import CollectionEditor from './CollectionEditor.svelte';
  import Field from './Field.svelte';
  import Notices from './Notices.svelte';
  import type { Dispatch, SectionView } from './types';
  let { section, number, dispatch }: { section: SectionView; number: number; dispatch: Dispatch } = $props();
</script>

{#snippet contents()}
  {#each section.fields as field (field.key)}
    <Field {field} {dispatch} />
  {/each}
  {#each section.collections as collection (collection.name)}
    <CollectionEditor {collection} {dispatch} />
  {/each}
  <Notices notices={section.notices} />
{/snippet}

{#if section.collapsed}
  <details class="cg-section cg-advanced" hidden={!section.visible}>
    <summary>{section.label}<span>{section.detail}</span></summary>
    {@render contents()}
  </details>
{:else}
  <section class="cg-section" hidden={!section.visible}>
    <h2><span class="cg-step">{String(number).padStart(2, '0')}</span>{section.label}</h2>
    {@render contents()}
  </section>
{/if}
