export default function RootLayout(props) {
  return (
    <>
      <Head>
        <title>App</title>
      </Head>
      {props.children}
    </>
  );
}
