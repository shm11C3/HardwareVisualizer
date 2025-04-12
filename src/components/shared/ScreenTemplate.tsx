interface ScreenTemplateProps {
  title?: string;
  children: React.ReactNode;
}

const ScreenTemplate: React.FC<ScreenTemplateProps> = ({ title, children }) => {
  return (
    <div className="mx-auto w-full px-4 pt-12 2xl:w-3/4">
      {title && <h2 className="py-3 font-bold text-3xl"> {title}</h2>}
      {children}
    </div>
  );
};

export default ScreenTemplate;
