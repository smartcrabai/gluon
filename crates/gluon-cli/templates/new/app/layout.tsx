export default function RootLayout({ children }: { children: any }) {
  return (
    <>
      <Head>
        <title>App</title>
      </Head>
      {children}
    </>
  );
}
