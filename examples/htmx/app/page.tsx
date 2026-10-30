export default function Page(props) {
  return (
    <main>
      <Head>
        <script src="https://unpkg.com/htmx.org@2.0.4"></script>
        <script src="https://unpkg.com/htmx-ext-sse@2.2.2"></script>
      </Head>
      <h1>{props.marker}</h1>
      <section hx-ext="sse" sse-connect="/events">
        <p sse-swap="query-progress">Waiting for query progress...</p>
        <p sse-swap="query-complete"></p>
      </section>
    </main>
  );
}
