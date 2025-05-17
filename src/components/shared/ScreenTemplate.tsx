interface ScreenTemplateProps {
  title?: string;
  children: React.ReactNode;
}

const ScreenTemplate: React.FC<ScreenTemplateProps> = ({ title, children }) => {
  return (
    <div className="mx-auto w-full pt-12 pr-4 pl-20 2xl:w-3/4 2xl:px-4">
      {title && <h2 className="py-3 font-bold text-3xl"> {title}</h2>}
      {children}
    </div>
  );
};

export default ScreenTemplate;
